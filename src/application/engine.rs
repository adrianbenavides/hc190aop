use crate::domain::account::{Balance, ClientAccount};
use crate::domain::ports::{AccountStore, TransactionStore};
use crate::domain::transaction::{DisputeStatus, Transaction, TransactionType};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug)]
enum EngineCommand {
    ProcessTransaction(Transaction),
    Shutdown,
}

pub struct PaymentEngine {
    sender: mpsc::Sender<EngineCommand>,
    handle: JoinHandle<()>,
}

impl PaymentEngine {
    pub fn new(
        account_store: Arc<dyn AccountStore>,
        transaction_store: Arc<dyn TransactionStore>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(1024);

        let mut worker = EngineWorker::new(account_store, transaction_store, receiver);
        let handle = tokio::spawn(async move {
            worker.run().await;
        });

        Self { sender, handle }
    }

    pub async fn process_transaction(
        &self,
        tx: Transaction,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.sender
            .send(EngineCommand::ProcessTransaction(tx))
            .await?;
        Ok(())
    }

    pub async fn shutdown(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.sender.send(EngineCommand::Shutdown).await?;
        self.handle.await?;
        Ok(())
    }
}

struct EngineWorker {
    account_store: Arc<dyn AccountStore>,
    transaction_store: Arc<dyn TransactionStore>,
    receiver: mpsc::Receiver<EngineCommand>,
}

impl EngineWorker {
    fn new(
        account_store: Arc<dyn AccountStore>,
        transaction_store: Arc<dyn TransactionStore>,
        receiver: mpsc::Receiver<EngineCommand>,
    ) -> Self {
        Self {
            account_store,
            transaction_store,
            receiver,
        }
    }

    async fn run(&mut self) {
        while let Some(command) = self.receiver.recv().await {
            match command {
                EngineCommand::ProcessTransaction(tx) => {
                    if let Err(e) = self.handle_transaction(tx).await {
                        eprintln!("Error processing transaction: {:?}", e);
                    }
                }
                EngineCommand::Shutdown => break,
            }
        }
    }

    async fn handle_transaction(
        &mut self,
        tx: Transaction,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
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
                    account.deposit(Balance(amount));
                    self.transaction_store.store(tx).await?;
                }
            }
            TransactionType::Withdrawal => {
                if let Some(amount) = tx.amount {
                    let _ = account.withdraw(Balance(amount));
                }
            }
            TransactionType::Dispute => {
                if let Some(mut original_tx) = self.transaction_store.get(tx.tx).await?
                    && original_tx.client == tx.client
                    && original_tx.dispute_status == DisputeStatus::None
                    && let Some(amount) = original_tx.amount
                    && account.hold(Balance(amount)).is_ok()
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
                    && account.resolve(Balance(amount)).is_ok()
                {
                    original_tx.dispute_status = DisputeStatus::None;
                    self.transaction_store.store(original_tx).await?;
                }
            }
            TransactionType::Chargeback => {
                if let Some(mut original_tx) = self.transaction_store.get(tx.tx).await?
                    && original_tx.client == tx.client
                    && original_tx.dispute_status == DisputeStatus::Disputed
                    && let Some(amount) = original_tx.amount
                    && account.chargeback(Balance(amount)).is_ok()
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
    use crate::infrastructure::in_memory::{InMemoryAccountStore, InMemoryTransactionStore};
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_payment_engine_shutdown() {
        let as_store = Arc::new(InMemoryAccountStore::new());
        let ts_store = Arc::new(InMemoryTransactionStore::new());

        let engine = PaymentEngine::new(as_store.clone(), ts_store.clone());

        let deposit = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0)),
            dispute_status: DisputeStatus::None,
        };

        engine.process_transaction(deposit).await.unwrap();

        // Shutdown should wait for processing to complete
        engine.shutdown().await.unwrap();

        let account = as_store.get(1).await.unwrap().unwrap();
        assert_eq!(account.available, Balance(dec!(100.0)));
    }
}
