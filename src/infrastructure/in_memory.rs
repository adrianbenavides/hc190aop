use crate::domain::account::{Amount, ClientAccount};
use crate::domain::ports::{AccountStore, TransactionStore};
use crate::domain::transaction::{DisputeStatus, Transaction, TransactionType};
use crate::error::Result;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// A minimalist representation of a transaction for in-memory storage.
///
/// Reduces RAM footprint by only storing fields essential for the dispute lifecycle.
#[derive(Clone, Copy)]
pub struct LeanTransaction {
    pub client_id: u16,
    pub amount: Amount,
    pub dispute_status: DisputeStatus,
}

/// A thread-safe in-memory store for client accounts.
///
/// Uses `Arc<RwLock<HashMap<u16, ClientAccount>>>` to allow shared concurrent access.
/// Ideal for testing or small datasets where persistence is not required.
#[derive(Default, Clone)]
pub struct InMemoryAccountStore {
    accounts: Arc<RwLock<HashMap<u16, ClientAccount>>>,
}

impl InMemoryAccountStore {
    /// Creates a new, empty in-memory account store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AccountStore for InMemoryAccountStore {
    async fn store(&self, account: ClientAccount) -> Result<()> {
        let mut accounts = self.accounts.write().await;
        accounts.insert(account.client, account);
        Ok(())
    }

    async fn get(&self, client_id: u16) -> Result<Option<ClientAccount>> {
        let accounts = self.accounts.read().await;
        Ok(accounts.get(&client_id).cloned())
    }

    async fn get_all(&self) -> Result<Vec<ClientAccount>> {
        let accounts = self.accounts.read().await;
        Ok(accounts.values().cloned().collect())
    }
}

/// A thread-safe in-memory store for transactions.
///
/// Uses `Arc<RwLock<...>>` for shared concurrent access.
/// Optimized for memory efficiency by:
/// 1. Only storing disputable transactions (Deposits) in the `records` map.
/// 2. Using `LeanTransaction` to minimize per-record overhead.
/// 3. Using a `seen_ids` set for global uniqueness tracking of all transaction types.
#[derive(Default, Clone)]
pub struct InMemoryTransactionStore {
    records: Arc<RwLock<HashMap<u32, LeanTransaction>>>,
    seen_ids: Arc<RwLock<HashSet<u32>>>,
}

impl InMemoryTransactionStore {
    /// Creates a new, empty in-memory transaction store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TransactionStore for InMemoryTransactionStore {
    async fn store(&self, tx: Transaction) -> Result<()> {
        let tx_id = tx.tx;
        let mut seen_ids = self.seen_ids.write().await;
        seen_ids.insert(tx_id);

        if tx.r#type == TransactionType::Deposit
            && let Some(amount) = tx.amount
        {
            let lean_tx = LeanTransaction {
                client_id: tx.client,
                amount,
                dispute_status: tx.dispute_status,
            };
            let mut records = self.records.write().await;
            records.insert(tx_id, lean_tx);
        }
        Ok(())
    }

    async fn get(&self, tx_id: u32) -> Result<Option<Transaction>> {
        let records = self.records.read().await;
        if let Some(lean) = records.get(&tx_id) {
            Ok(Some(Transaction {
                r#type: TransactionType::Deposit,
                client: lean.client_id,
                tx: tx_id,
                amount: Some(lean.amount),
                dispute_status: lean.dispute_status,
            }))
        } else {
            Ok(None)
        }
    }

    async fn exists(&self, tx_id: u32) -> Result<bool> {
        let seen_ids = self.seen_ids.read().await;
        Ok(seen_ids.contains(&tx_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::Balance;
    use crate::domain::transaction::TransactionType;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_in_memory_account_store() {
        let store = InMemoryAccountStore::new();
        let mut account = ClientAccount::new(1);
        account.available = Balance::new(dec!(100.0));

        store.store(account.clone()).await.unwrap();
        let retrieved = store.get(1).await.unwrap().unwrap();
        assert_eq!(retrieved, account);

        assert!(store.get(2).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_in_memory_account_store_get_all_accounts() {
        let store = InMemoryAccountStore::new();
        let account1 = ClientAccount::new(1);
        let account2 = ClientAccount::new(2);
        store.store(account1.clone()).await.unwrap();
        store.store(account2.clone()).await.unwrap();

        let all = store.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&account1));
        assert!(all.contains(&account2));
    }

    #[tokio::test]
    async fn test_in_memory_transaction_store() {
        let store = InMemoryTransactionStore::new();
        let tx = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0).try_into().unwrap()),
            dispute_status: Default::default(),
        };

        store.store(tx.clone()).await.unwrap();
        let retrieved = store.get(1).await.unwrap().unwrap();
        assert_eq!(retrieved, tx);
    }

    #[tokio::test]
    async fn test_uniqueness() {
        let store = InMemoryTransactionStore::new();

        let deposit = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0).try_into().unwrap()),
            dispute_status: Default::default(),
        };
        let withdrawal = Transaction {
            r#type: TransactionType::Withdrawal,
            client: 1,
            tx: 2,
            amount: Some(dec!(50.0).try_into().unwrap()),
            dispute_status: Default::default(),
        };

        store.store(deposit.clone()).await.unwrap();
        store.store(withdrawal.clone()).await.unwrap();

        // 1. Uniqueness check: Both should be seen
        assert!(store.exists(1).await.unwrap());
        assert!(store.exists(2).await.unwrap());

        // 2. Selective storage: Deposit should be in records, withdrawal should NOT
        assert!(store.get(1).await.unwrap().is_some());
        assert!(store.get(2).await.unwrap().is_none());

        // 3. Lean record reconstruction
        let retrieved_deposit = store.get(1).await.unwrap().unwrap();
        assert_eq!(retrieved_deposit.client, 1);
        assert_eq!(retrieved_deposit.tx, 1);
        assert_eq!(retrieved_deposit.amount, deposit.amount);
    }
}
