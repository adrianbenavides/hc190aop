use crate::domain::account::ClientAccount;
use crate::domain::ports::{AccountStore, TransactionStore};
use crate::domain::transaction::Transaction;
use async_trait::async_trait;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default, Clone)]
pub struct InMemoryAccountStore {
    accounts: Arc<RwLock<HashMap<u16, ClientAccount>>>,
}

impl InMemoryAccountStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AccountStore for InMemoryAccountStore {
    async fn store(&self, account: ClientAccount) -> io::Result<()> {
        let mut accounts = self.accounts.write().await;
        accounts.insert(account.client, account);
        Ok(())
    }

    async fn get(&self, client_id: u16) -> io::Result<Option<ClientAccount>> {
        let accounts = self.accounts.read().await;
        Ok(accounts.get(&client_id).cloned())
    }

    async fn all_accounts(&self) -> io::Result<Vec<ClientAccount>> {
        let accounts = self.accounts.read().await;
        Ok(accounts.values().cloned().collect())
    }
}

#[derive(Default, Clone)]
pub struct InMemoryTransactionStore {
    transactions: Arc<RwLock<HashMap<u32, Transaction>>>,
}

impl InMemoryTransactionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TransactionStore for InMemoryTransactionStore {
    async fn store(&self, tx: Transaction) -> io::Result<()> {
        let mut transactions = self.transactions.write().await;
        transactions.insert(tx.tx, tx);
        Ok(())
    }

    async fn get(&self, tx_id: u32) -> io::Result<Option<Transaction>> {
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
    async fn test_in_memory_transaction_store() {
        let store = InMemoryTransactionStore::new();
        let tx = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0)),
            dispute_status: Default::default(),
        };

        store.store(tx.clone()).await.unwrap();
        let retrieved = store.get(1).await.unwrap().unwrap();
        assert_eq!(retrieved, tx);
    }
}
