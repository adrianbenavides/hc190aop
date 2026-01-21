use crate::domain::account::ClientAccount;
use crate::domain::ports::{AccountStore, TransactionStore};
use crate::domain::transaction::Transaction;
use async_trait::async_trait;
use rocksdb::{ColumnFamilyDescriptor, DB, Options};
use std::io;
use std::path::Path;
use std::sync::Arc;

pub const CF_ACCOUNTS: &str = "accounts";
pub const CF_TRANSACTIONS: &str = "transactions";

#[derive(Clone)]
pub struct RocksDBStore {
    db: Arc<DB>,
}

impl RocksDBStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, rocksdb::Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_accounts = ColumnFamilyDescriptor::new(CF_ACCOUNTS, Options::default());
        let cf_transactions = ColumnFamilyDescriptor::new(CF_TRANSACTIONS, Options::default());

        let db = DB::open_cf_descriptors(&opts, path, vec![cf_accounts, cf_transactions])?;

        Ok(Self { db: Arc::new(db) })
    }
}

#[async_trait]
impl AccountStore for RocksDBStore {
    async fn store(&self, account: ClientAccount) -> io::Result<()> {
        let cf = self.db.cf_handle(CF_ACCOUNTS).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Accounts column family not found")
        })?;

        let key = account.client.to_be_bytes();
        let value = serde_json::to_vec(&account)
            .map_err(|e| io::Error::other(format!("Serialization error: {}", e)))?;

        self.db
            .put_cf(&cf, key, value)
            .map_err(|e| io::Error::other(format!("RocksDB error: {}", e)))?;

        Ok(())
    }

    async fn get(&self, client_id: u16) -> io::Result<Option<ClientAccount>> {
        let cf = self.db.cf_handle(CF_ACCOUNTS).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Accounts column family not found")
        })?;

        let key = client_id.to_be_bytes();
        let result = self
            .db
            .get_cf(&cf, key)
            .map_err(|e| io::Error::other(format!("RocksDB error: {}", e)))?;

        if let Some(bytes) = result {
            let account = serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::other(format!("Deserialization error: {}", e)))?;
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    async fn get_all(&self, client_id: u16) -> io::Result<Vec<ClientAccount>> {
        AccountStore::get(self, client_id)
            .await
            .map(|opt| opt.into_iter().collect())
    }
}

#[async_trait]
impl TransactionStore for RocksDBStore {
    async fn store(&self, tx: Transaction) -> io::Result<()> {
        let cf = self.db.cf_handle(CF_TRANSACTIONS).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Transactions column family not found",
            )
        })?;

        let key = tx.tx.to_be_bytes();
        let value = serde_json::to_vec(&tx)
            .map_err(|e| io::Error::other(format!("Serialization error: {}", e)))?;

        self.db
            .put_cf(&cf, key, value)
            .map_err(|e| io::Error::other(format!("RocksDB error: {}", e)))?;

        Ok(())
    }

    async fn get(&self, tx_id: u32) -> io::Result<Option<Transaction>> {
        let cf = self.db.cf_handle(CF_TRANSACTIONS).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Transactions column family not found",
            )
        })?;

        let key = tx_id.to_be_bytes();
        let result = self
            .db
            .get_cf(&cf, key)
            .map_err(|e| io::Error::other(format!("RocksDB error: {}", e)))?;

        if let Some(bytes) = result {
            let tx = serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::other(format!("Deserialization error: {}", e)))?;
            Ok(Some(tx))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::Balance;
    use crate::domain::transaction::{DisputeStatus, TransactionType};
    use rust_decimal_macros::dec;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_rocksdb_open_cf() {
        let dir = tempdir().unwrap();
        let store = RocksDBStore::open(dir.path()).expect("Failed to open RocksDB");

        // Verify CFs exist
        assert!(store.db.cf_handle(CF_ACCOUNTS).is_some());
        assert!(store.db.cf_handle(CF_TRANSACTIONS).is_some());
    }

    #[tokio::test]
    async fn test_rocksdb_account_store() {
        let dir = tempdir().unwrap();
        let store = RocksDBStore::open(dir.path()).unwrap();

        let mut account = ClientAccount::new(1);
        account.available = Balance::new(dec!(100.0));

        AccountStore::store(&store, account.clone()).await.unwrap();

        let retrieved = AccountStore::get(&store, 1).await.unwrap().unwrap();
        assert_eq!(retrieved, account);

        let all = AccountStore::get_all(&store, 1).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], account);

        assert!(AccountStore::get(&store, 2).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_rocksdb_transaction_store() {
        let dir = tempdir().unwrap();
        let store = RocksDBStore::open(dir.path()).unwrap();

        let tx = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0)),
            dispute_status: DisputeStatus::None,
        };

        TransactionStore::store(&store, tx.clone()).await.unwrap();

        let retrieved = TransactionStore::get(&store, 1).await.unwrap().unwrap();
        assert_eq!(retrieved, tx);

        assert!(TransactionStore::get(&store, 2).await.unwrap().is_none());
    }
}
