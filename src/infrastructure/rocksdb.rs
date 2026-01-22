use crate::domain::account::ClientAccount;
use crate::domain::ports::{AccountStore, TransactionStore};
use crate::domain::transaction::Transaction;
use crate::error::{PaymentError, Result};
use async_trait::async_trait;
use rocksdb::{ColumnFamilyDescriptor, DB, Options};
use std::path::Path;
use std::sync::Arc;

/// Column Family for storing account states.
pub const CF_ACCOUNTS: &str = "accounts";
/// Column Family for storing transaction history.
pub const CF_TRANSACTIONS: &str = "transactions";

/// A persistent store implementation using RocksDB.
///
/// Handles storage for both `ClientAccount` and `Transaction` entities using
/// separate Column Families. This ensures data separation and efficient retrieval.
///
/// This struct is thread-safe (`Clone` shares the underlying `Arc<DB>`).
#[derive(Clone)]
pub struct RocksDBStore {
    db: Arc<DB>,
}

impl RocksDBStore {
    /// Opens or creates a RocksDB instance at the specified path.
    ///
    /// Ensures that the required column families ("accounts" and "transactions") exist.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path where the database will be stored.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
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
    async fn store(&self, account: ClientAccount) -> Result<()> {
        let cf = self.db.cf_handle(CF_ACCOUNTS).ok_or_else(|| {
            PaymentError::InternalError(Box::new(std::io::Error::other(
                "Accounts column family not found",
            )))
        })?;

        let key = account.client.to_be_bytes();
        let value = serde_json::to_vec(&account).map_err(|e| {
            PaymentError::InternalError(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Serialization error: {}", e),
            )))
        })?;

        self.db.put_cf(&cf, key, value)?;

        Ok(())
    }

    async fn get(&self, client_id: u16) -> Result<Option<ClientAccount>> {
        let cf = self.db.cf_handle(CF_ACCOUNTS).ok_or_else(|| {
            PaymentError::InternalError(Box::new(std::io::Error::other(
                "Accounts column family not found",
            )))
        })?;

        let key = client_id.to_be_bytes();
        let result = self.db.get_cf(&cf, key)?;

        if let Some(bytes) = result {
            let account = serde_json::from_slice(&bytes).map_err(|e| {
                PaymentError::InternalError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Deserialization error: {}", e),
                )))
            })?;
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    async fn get_all(&self) -> Result<Vec<ClientAccount>> {
        let handle = self.db.cf_handle("accounts").ok_or_else(|| {
            PaymentError::InternalError(Box::new(std::io::Error::other(
                "Accounts column family not found",
            )))
        })?;

        let mut accounts = Vec::new();
        let iter = self.db.iterator_cf(handle, rocksdb::IteratorMode::Start);

        for item in iter {
            let (_key, value) = item.map_err(|e| {
                PaymentError::InternalError(Box::new(std::io::Error::other(format!(
                    "RocksDB iteration error: {}",
                    e
                ))))
            })?;
            let account: ClientAccount = serde_json::from_slice(&value).map_err(|e| {
                PaymentError::InternalError(Box::new(std::io::Error::other(format!(
                    "Failed to deserialize account: {}",
                    e
                ))))
            })?;
            accounts.push(account);
        }

        Ok(accounts)
    }
}

#[async_trait]
impl TransactionStore for RocksDBStore {
    async fn store(&self, tx: Transaction) -> Result<()> {
        let cf = self.db.cf_handle(CF_TRANSACTIONS).ok_or_else(|| {
            PaymentError::InternalError(Box::new(std::io::Error::other(
                "Transactions column family not found",
            )))
        })?;

        let key = tx.tx.to_be_bytes();
        let value = serde_json::to_vec(&tx).map_err(|e| {
            PaymentError::InternalError(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Serialization error: {}", e),
            )))
        })?;

        self.db.put_cf(&cf, key, value)?;

        Ok(())
    }

    async fn get(&self, tx_id: u32) -> Result<Option<Transaction>> {
        let cf = self.db.cf_handle(CF_TRANSACTIONS).ok_or_else(|| {
            PaymentError::InternalError(Box::new(std::io::Error::other(
                "Transactions column family not found",
            )))
        })?;

        let key = tx_id.to_be_bytes();
        let result = self.db.get_cf(&cf, key)?;

        if let Some(bytes) = result {
            let tx = serde_json::from_slice(&bytes).map_err(|e| {
                PaymentError::InternalError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Deserialization error: {}", e),
                )))
            })?;
            Ok(Some(tx))
        } else {
            Ok(None)
        }
    }

    async fn exists(&self, tx_id: u32) -> Result<bool> {
        let cf = self.db.cf_handle(CF_TRANSACTIONS).ok_or_else(|| {
            PaymentError::InternalError(Box::new(std::io::Error::other(
                "Transactions column family not found",
            )))
        })?;

        let key = tx_id.to_be_bytes();
        // Just check if the key exists without retrieving the value
        let result = self.db.get_pinned_cf(&cf, key)?;
        Ok(result.is_some())
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

        let all = AccountStore::get_all(&store).await.unwrap();
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
            amount: Some(dec!(100.0).try_into().unwrap()),
            dispute_status: DisputeStatus::None,
        };

        TransactionStore::store(&store, tx.clone()).await.unwrap();

        let retrieved = TransactionStore::get(&store, 1).await.unwrap().unwrap();
        assert_eq!(retrieved, tx);

        assert!(TransactionStore::get(&store, 2).await.unwrap().is_none());
    }
}
