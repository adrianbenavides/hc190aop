use crate::domain::account::{Balance, ClientAccount};
use crate::domain::ports::{
    AccountStoreBox, AccountStoreFactory, TransactionStoreBox, TransactionStoreFactory,
};
use crate::domain::transaction::{DisputeStatus, Transaction, TransactionType};
use std::collections::HashMap;
use std::error::Error;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug)]
enum EngineCommand {
    ProcessTransaction(Transaction),
    Shutdown,
}

pub struct PaymentEngine {
    sender: mpsc::Sender<EngineCommand>,
    handle: JoinHandle<Result<Vec<ClientAccount>, Box<dyn Error + Send + Sync>>>,
}

impl PaymentEngine {
    pub fn new(
        account_factory: AccountStoreFactory,
        transaction_factory: TransactionStoreFactory,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(1024);

        let mut router = RouterWorker::new(account_factory, transaction_factory, receiver);
        let handle = tokio::spawn(async move { router.run().await });

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

    pub async fn shutdown(self) -> Result<Vec<ClientAccount>, Box<dyn Error + Send + Sync>> {
        self.sender.send(EngineCommand::Shutdown).await?;
        self.handle.await?
    }
}

struct RouterWorker {
    account_factory: AccountStoreFactory,
    transaction_factory: TransactionStoreFactory,
    receiver: mpsc::Receiver<EngineCommand>,
    workers: HashMap<u16, mpsc::Sender<EngineCommand>>,
    worker_handles: Vec<(u16, JoinHandle<AccountStoreBox>)>,
}

impl RouterWorker {
    fn new(
        account_factory: AccountStoreFactory,
        transaction_factory: TransactionStoreFactory,
        receiver: mpsc::Receiver<EngineCommand>,
    ) -> Self {
        Self {
            account_factory,
            transaction_factory,
            receiver,
            workers: HashMap::new(),
            worker_handles: Vec::new(),
        }
    }

    async fn run(&mut self) -> Result<Vec<ClientAccount>, Box<dyn Error + Send + Sync>> {
        while let Some(command) = self.receiver.recv().await {
            match command {
                EngineCommand::ProcessTransaction(tx) => {
                    let client_id = tx.client;
                    let worker_sender = if let Some(sender) = self.workers.get(&client_id) {
                        sender.clone()
                    } else {
                        let (ws, wr) = mpsc::channel(128);
                        let worker = ClientWorker::new(
                            client_id,
                            (self.account_factory)(),
                            (self.transaction_factory)(),
                            wr,
                        );
                        let handle = tokio::spawn(async move { worker.run().await });
                        self.worker_handles.push((client_id, handle));
                        self.workers.insert(client_id, ws.clone());
                        ws
                    };
                    let _ = worker_sender
                        .send(EngineCommand::ProcessTransaction(tx))
                        .await;
                }
                EngineCommand::Shutdown => break,
            }
        }

        // Shutdown all workers
        for sender in self.workers.values() {
            let _ = sender.send(EngineCommand::Shutdown).await;
        }

        // Aggregate results from stores
        let mut final_accounts = Vec::new();
        for (client_id, handle) in self.worker_handles.drain(..) {
            let store = handle.await?;
            let accounts = store.get_all(client_id).await?;
            final_accounts.extend(accounts);
        }

        Ok(final_accounts)
    }
}

struct ClientWorker {
    client_id: u16,
    account_store: AccountStoreBox,
    transaction_store: TransactionStoreBox,
    receiver: mpsc::Receiver<EngineCommand>,
}

impl ClientWorker {
    fn new(
        client_id: u16,
        account_store: AccountStoreBox,
        transaction_store: TransactionStoreBox,
        receiver: mpsc::Receiver<EngineCommand>,
    ) -> Self {
        Self {
            client_id,
            account_store,
            transaction_store,
            receiver,
        }
    }

    async fn run(mut self) -> AccountStoreBox {
        while let Some(command) = self.receiver.recv().await {
            match command {
                EngineCommand::ProcessTransaction(tx) => {
                    if let Err(e) = self.handle_transaction(tx).await {
                        eprintln!(
                            "Error processing transaction for client {}: {:?}",
                            self.client_id, e
                        );
                    }
                }
                EngineCommand::Shutdown => break,
            }
        }
        self.account_store
    }

    async fn handle_transaction(
        &mut self,
        tx: Transaction,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut account = self
            .account_store
            .get(self.client_id)
            .await?
            .unwrap_or_else(|| ClientAccount::new(self.client_id));

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
    async fn test_client_worker_processing() {
        let (ws, wr) = mpsc::channel(10);

        let worker = ClientWorker::new(
            1,
            Box::new(InMemoryAccountStore::new()),
            Box::new(InMemoryTransactionStore::new()),
            wr,
        );

        let deposit = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0)),
            dispute_status: DisputeStatus::None,
        };

        ws.send(EngineCommand::ProcessTransaction(deposit))
            .await
            .unwrap();
        ws.send(EngineCommand::Shutdown).await.unwrap();

        let store = worker.run().await;
        let final_account = store.get(1).await.unwrap().unwrap();
        assert_eq!(final_account.available, Balance(dec!(100.0)));
    }

    #[tokio::test]
    async fn test_payment_engine_aggregation() {
        let af: AccountStoreFactory = Box::new(|| Box::new(InMemoryAccountStore::new()));
        let tf: TransactionStoreFactory = Box::new(|| Box::new(InMemoryTransactionStore::new()));

        let engine = PaymentEngine::new(af, tf);

        // Send deposits for 100 different clients
        for i in 1..=100 {
            let tx = Transaction {
                r#type: TransactionType::Deposit,
                client: i as u16,
                tx: i,
                amount: Some(dec!(1.0)),
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
}
