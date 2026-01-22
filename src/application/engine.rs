use crate::domain::account::ClientAccount;
use crate::domain::ports::{AccountStoreBox, TransactionStoreBox};
use crate::domain::transaction::{DisputeStatus, Transaction, TransactionType};
use crate::error::Result;

/// The main entry point for the transaction processing application.
///
/// `PaymentEngine` handles the processing of financial transactions.
/// It owns the storage backends and ensures sequential consistency by awaiting
/// storage operations for each transaction.
pub struct PaymentEngine {
    account_store: AccountStoreBox,
    transaction_store: TransactionStoreBox,
}

impl PaymentEngine {
    /// Creates a new `PaymentEngine` instance.
    ///
    /// # Arguments
    ///
    /// * `account_store` - The store for client accounts.
    /// * `transaction_store` - The store for transaction history.
    pub fn new(account_store: AccountStoreBox, transaction_store: TransactionStoreBox) -> Self {
        Self {
            account_store,
            transaction_store,
        }
    }

    /// Submits a transaction for processing.
    ///
    /// This method processes the transaction and persists the results directly.
    /// It ensures sequential consistency by awaiting storage operations.
    pub async fn process_transaction(&self, tx: Transaction) -> Result<()> {
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
                    if !self.transaction_store.exists(tx.tx).await? {
                        account.deposit(amount.into());
                        self.transaction_store.store(tx).await?;
                    }
                }
            }
            TransactionType::Withdrawal => {
                if let Some(amount) = tx.amount {
                    // Ignore duplicate transaction IDs
                    if !self.transaction_store.exists(tx.tx).await? {
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

    /// Consumes the engine and returns the final state of all accounts.
    pub async fn into_results(self) -> Result<Vec<ClientAccount>> {
        self.account_store.get_all().await
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
        let as_store = Box::new(InMemoryAccountStore::new());
        let ts_store = Box::new(InMemoryTransactionStore::new());

        let engine = PaymentEngine::new(as_store, ts_store);

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

        engine.process_transaction(deposit1).await.unwrap();
        engine.process_transaction(deposit2).await.unwrap();

        let results = engine.into_results().await.unwrap();
        let final_account = results.iter().find(|a| a.client == 1).unwrap();
        // Should be 100.0, not 150.0
        assert_eq!(final_account.available, Balance(dec!(100.0)));
    }

    #[tokio::test]
    async fn test_client_worker_processing() {
        let engine = PaymentEngine::new(
            Box::new(InMemoryAccountStore::new()),
            Box::new(InMemoryTransactionStore::new()),
        );

        let deposit = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0).try_into().unwrap()),
            dispute_status: DisputeStatus::None,
        };

        engine.process_transaction(deposit).await.unwrap();

        let results = engine.into_results().await.unwrap();
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

        // into_results should return all 100 accounts
        let results = engine.into_results().await.unwrap();
        assert_eq!(results.len(), 100);

        for account in results {
            assert_eq!(account.available, Balance(dec!(1.0)));
        }
    }

    #[tokio::test]
    async fn test_dispute_finality() {
        let engine = PaymentEngine::new(
            Box::new(InMemoryAccountStore::new()),
            Box::new(InMemoryTransactionStore::new()),
        );

        // 1. Deposit
        let deposit = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0).try_into().unwrap()),
            dispute_status: DisputeStatus::None,
        };
        engine.process_transaction(deposit).await.unwrap();

        // 2. Dispute
        let dispute = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: None,
            dispute_status: DisputeStatus::None,
        };
        engine.process_transaction(dispute.clone()).await.unwrap();

        // 3. Resolve
        let resolve = Transaction {
            r#type: TransactionType::Resolve,
            client: 1,
            tx: 1,
            amount: None,
            dispute_status: DisputeStatus::None,
        };
        engine.process_transaction(resolve).await.unwrap();

        // 4. Try to Dispute Again (Should fail/be ignored)
        engine.process_transaction(dispute).await.unwrap();

        let results = engine.into_results().await.unwrap();
        let account = results.iter().find(|a| a.client == 1).unwrap();

        // Account should be fully available (100.0), nothing held.
        // If re-dispute succeeded, 100.0 would be held.
        assert_eq!(account.available, Balance(dec!(100.0)));
        assert_eq!(account.held, Balance(dec!(0.0)));
    }
}
