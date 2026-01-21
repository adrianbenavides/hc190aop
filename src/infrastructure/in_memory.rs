use crate::domain::account::ClientAccount;
use crate::domain::ports::{AccountStore, TransactionStore};
use crate::domain::transaction::Transaction;
use crate::error::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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

    async fn get_all(&self, client_id: u16) -> Result<Vec<ClientAccount>> {
        let accounts = self.accounts.read().await;
        Ok(accounts.get(&client_id).into_iter().cloned().collect())
    }
}

/// A thread-safe in-memory store for transactions.
///
/// Uses `Arc<RwLock<HashMap<u32, Transaction>>>` for shared concurrent access.
/// Essential for validating and processing disputes against transaction history.
#[derive(Default, Clone)]
pub struct InMemoryTransactionStore {
    transactions: Arc<RwLock<HashMap<u32, Transaction>>>,
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
        let mut transactions = self.transactions.write().await;
        transactions.insert(tx.tx, tx);
        Ok(())
    }

    async fn get(&self, tx_id: u32) -> Result<Option<Transaction>> {
        let transactions = self.transactions.read().await;
        Ok(transactions.get(&tx_id).cloned())
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
    async fn test_in_memory_account_store_get_all() {
        let store = InMemoryAccountStore::new();
        let account = ClientAccount::new(1);
        store.store(account.clone()).await.unwrap();

        let all = store.get_all(1).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], account);

        let empty = store.get_all(2).await.unwrap();
        assert_eq!(empty.len(), 0);
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
}
