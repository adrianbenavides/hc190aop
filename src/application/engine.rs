use crate::domain::account::ClientAccount;
use crate::domain::ports::{AccountStoreBox, TransactionStoreBox};
use crate::domain::transaction::{DisputeStatus, Transaction, TransactionType};
use crate::error::{PaymentError, Result};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Commands sent to the payment engine for processing or control.
#[derive(Debug)]
enum EngineCommand {
    /// Process a new transaction.
    ProcessTransaction(Transaction),
    /// Gracefully shutdown the engine and return results.
    Shutdown,
}

/// The main entry point for the transaction processing application.
///
/// `PaymentEngine` orchestrates the flow of transactions using an Actor model.
/// It delegates processing to a background `EngineWorker` via a channel, ensuring
/// non-blocking operation for the caller.
pub struct PaymentEngine {
    sender: mpsc::Sender<EngineCommand>,
    handle: JoinHandle<Result<Vec<ClientAccount>>>,
}

impl PaymentEngine {
    /// Creates a new `PaymentEngine` instance.
    ///
    /// Spawns a background `EngineWorker` task to handle incoming commands.
    ///
    /// # Arguments
    ///
    /// * `account_store` - The store for client accounts.
    /// * `transaction_store` - The store for transaction history.
    pub fn new(account_store: AccountStoreBox, transaction_store: TransactionStoreBox) -> Self {
        let (sender, receiver) = mpsc::channel(1024);

        let worker = EngineWorker::new(account_store, transaction_store, receiver);
        let handle = tokio::spawn(async move { worker.run().await });

        Self { sender, handle }
    }

    /// Submits a transaction for asynchronous processing.
    ///
    /// This method sends the transaction to the background worker via a channel.
    /// It returns immediately, not waiting for the transaction to be processed.
    pub async fn process_transaction(&self, tx: Transaction) -> Result<()> {
        self.sender
            .send(EngineCommand::ProcessTransaction(tx))
            .await
            .map_err(|_| {
                PaymentError::InternalError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "Engine channel closed",
                )))
            })?;
        Ok(())
    }

    /// Signals the engine to shutdown and awaits the final results.
    ///
    /// This method:
    /// 1. Sends a shutdown command to the worker.
    /// 2. Waits for the worker to finish processing pending messages.
    /// 3. Returns the aggregated list of all client accounts.
    pub async fn shutdown(self) -> Result<Vec<ClientAccount>> {
        self.sender
            .send(EngineCommand::Shutdown)
            .await
            .map_err(|_| {
                PaymentError::InternalError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "Engine channel closed",
                )))
            })?;
        self.handle.await?
    }
}

struct EngineWorker {
    account_store: AccountStoreBox,
    transaction_store: TransactionStoreBox,
    receiver: mpsc::Receiver<EngineCommand>,
}

impl EngineWorker {
    fn new(
        account_store: AccountStoreBox,
        transaction_store: TransactionStoreBox,
        receiver: mpsc::Receiver<EngineCommand>,
    ) -> Self {
        Self {
            account_store,
            transaction_store,
            receiver,
        }
    }

    async fn run(mut self) -> Result<Vec<ClientAccount>> {
        while let Some(command) = self.receiver.recv().await {
            match command {
                EngineCommand::ProcessTransaction(tx) => {
                    if let Err(e) = self.handle_transaction(tx).await {
                        // In a real system, we might log this to a file or monitoring system.
                        // For this CLI tool, we can print to stderr.
                        eprintln!("Error processing transaction: {:?}", e);
                    }
                }
                EngineCommand::Shutdown => break,
            }
        }

        // Aggregate results from stores
        self.account_store.get_all().await
    }

    async fn handle_transaction(&mut self, tx: Transaction) -> Result<()> {
        let mut account = self
            .account_store
            .get(tx.client)
            .await?
            .unwrap_or_else(|| ClientAccount::new(tx.client));

        // Skip if account is locked
        if account.status == crate::domain::account::AccountStatus::Locked {
            return Ok(());
        }

        match tx.r#type {
            TransactionType::Deposit => {
                if let Some(amount) = tx.amount {
                    // Ignore duplicate transaction IDs
                    if self.transaction_store.get(tx.tx).await?.is_none() {
                        account.deposit(amount.into());
                        self.transaction_store.store(tx).await?;
                    }
                }
            }
            TransactionType::Withdrawal => {
                if let Some(amount) = tx.amount {
                    // Ignore duplicate transaction IDs
                    if self.transaction_store.get(tx.tx).await?.is_none() {
                        let _ = account.withdraw(amount.into());
                        self.transaction_store.store(tx).await?;
                    }
                }
            }
            TransactionType::Dispute => {
                if let Some(mut original_tx) = self.transaction_store.get(tx.tx).await?
                    && original_tx.r#type == TransactionType::Deposit
                    && original_tx.client == tx.client
                    && original_tx.dispute_status == DisputeStatus::None
                    && let Some(amount) = original_tx.amount
                    && account.hold(amount.into()).is_ok()
                {
                    original_tx.dispute_status = DisputeStatus::Disputed;
                    self.transaction_store.store(original_tx).await?;
                }
            }
            TransactionType::Resolve => {
                if let Some(mut original_tx) = self.transaction_store.get(tx.tx).await?
                    && original_tx.client == tx.client
                    && original_tx.dispute_status == DisputeStatus::Disputed
                    && let Some(amount) = original_tx.amount
                    && account.resolve(amount.into()).is_ok()
                {
                    original_tx.dispute_status = DisputeStatus::Resolved;
                    self.transaction_store.store(original_tx).await?;
                }
            }
            TransactionType::Chargeback => {
                if let Some(mut original_tx) = self.transaction_store.get(tx.tx).await?
                    && original_tx.client == tx.client
                    && original_tx.dispute_status == DisputeStatus::Disputed
                    && let Some(amount) = original_tx.amount
                    && account.chargeback(amount.into()).is_ok()
                {
                    original_tx.dispute_status = DisputeStatus::Chargebacked;
                    self.transaction_store.store(original_tx).await?;
                }
            }
        }

        self.account_store.store(account).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::Balance;
    use crate::infrastructure::in_memory::{InMemoryAccountStore, InMemoryTransactionStore};
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_duplicate_transaction_ids() {
        let (ws, wr) = mpsc::channel(10);
        let as_store = Box::new(InMemoryAccountStore::new());
        let ts_store = Box::new(InMemoryTransactionStore::new());

        let worker = EngineWorker::new(as_store, ts_store, wr);

        let deposit1 = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0).try_into().unwrap()),
            dispute_status: DisputeStatus::None,
        };
        let deposit2 = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1, // Duplicate ID
            amount: Some(dec!(50.0).try_into().unwrap()),
            dispute_status: DisputeStatus::None,
        };

        ws.send(EngineCommand::ProcessTransaction(deposit1))
            .await
            .unwrap();
        ws.send(EngineCommand::ProcessTransaction(deposit2))
            .await
            .unwrap();
        ws.send(EngineCommand::Shutdown).await.unwrap();

        let results = worker.run().await.unwrap();
        let final_account = results.iter().find(|a| a.client == 1).unwrap();
        // Should be 100.0, not 150.0
        assert_eq!(final_account.available, Balance(dec!(100.0)));
    }

    #[tokio::test]
    async fn test_client_worker_processing() {
        let (ws, wr) = mpsc::channel(10);

        let worker = EngineWorker::new(
            Box::new(InMemoryAccountStore::new()),
            Box::new(InMemoryTransactionStore::new()),
            wr,
        );

        let deposit = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0).try_into().unwrap()),
            dispute_status: DisputeStatus::None,
        };

        ws.send(EngineCommand::ProcessTransaction(deposit))
            .await
            .unwrap();
        ws.send(EngineCommand::Shutdown).await.unwrap();

        let results = worker.run().await.unwrap();
        let final_account = results.iter().find(|a| a.client == 1).unwrap();
        assert_eq!(final_account.available, Balance(dec!(100.0)));
    }

    #[tokio::test]
    async fn test_payment_engine_aggregation() {
        let as_store: AccountStoreBox = Box::new(InMemoryAccountStore::new());
        let ts_store: TransactionStoreBox = Box::new(InMemoryTransactionStore::new());

        let engine = PaymentEngine::new(as_store, ts_store);

        // Send deposits for 100 different clients
        for i in 1..=100 {
            let tx = Transaction {
                r#type: TransactionType::Deposit,
                client: i as u16,
                tx: i,
                amount: Some(dec!(1.0).try_into().unwrap()),
                dispute_status: DisputeStatus::None,
            };
            engine.process_transaction(tx).await.unwrap();
        }

        // Shutdown should return all 100 accounts
        let results = engine.shutdown().await.unwrap();
        assert_eq!(results.len(), 100);

        for account in results {
            assert_eq!(account.available, Balance(dec!(1.0)));
        }
    }

    #[tokio::test]
    async fn test_dispute_finality() {
        let (ws, wr) = mpsc::channel(10);
        let worker = EngineWorker::new(
            Box::new(InMemoryAccountStore::new()),
            Box::new(InMemoryTransactionStore::new()),
            wr,
        );

        // 1. Deposit
        let deposit = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0).try_into().unwrap()),
            dispute_status: DisputeStatus::None,
        };
        ws.send(EngineCommand::ProcessTransaction(deposit))
            .await
            .unwrap();

        // 2. Dispute
        let dispute = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: None,
            dispute_status: DisputeStatus::None,
        };
        ws.send(EngineCommand::ProcessTransaction(dispute.clone()))
            .await
            .unwrap();

        // 3. Resolve
        let resolve = Transaction {
            r#type: TransactionType::Resolve,
            client: 1,
            tx: 1,
            amount: None,
            dispute_status: DisputeStatus::None,
        };
        ws.send(EngineCommand::ProcessTransaction(resolve))
            .await
            .unwrap();

        // 4. Try to Dispute Again (Should fail/be ignored)
        ws.send(EngineCommand::ProcessTransaction(dispute))
            .await
            .unwrap();

        ws.send(EngineCommand::Shutdown).await.unwrap();

        let results = worker.run().await.unwrap();
        let account = results.iter().find(|a| a.client == 1).unwrap();

        // Account should be fully available (100.0), nothing held.
        // If re-dispute succeeded, 100.0 would be held.
        assert_eq!(account.available, Balance(dec!(100.0)));
        assert_eq!(account.held, Balance(dec!(0.0)));
    }
}
