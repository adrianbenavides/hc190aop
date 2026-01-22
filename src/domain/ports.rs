use super::account::ClientAccount;
use super::transaction::Transaction;
use crate::error::Result;
use async_trait::async_trait;

#[async_trait]
/// Interface for persisting and retrieving client account states.
pub trait AccountStore: Send + Sync {
    /// Persists the current state of a client account.
    async fn store(&self, account: ClientAccount) -> Result<()>;
    /// Retrieves a client account by ID.
    async fn get(&self, client_id: u16) -> Result<Option<ClientAccount>>;
    /// Retrieves all client accounts currently in the store.
    async fn get_all(&self) -> Result<Vec<ClientAccount>>;
}

#[async_trait]
/// Interface for persisting and retrieving transaction history.
///
/// This is crucial for handling disputes, which reference past transactions by ID.
pub trait TransactionStore: Send + Sync {
    /// Stores a transaction record.
    async fn store(&self, tx: Transaction) -> Result<()>;
    /// Retrieves a transaction by its global ID.
    async fn get(&self, tx_id: u32) -> Result<Option<Transaction>>;
}

pub type AccountStoreBox = Box<dyn AccountStore>;
pub type TransactionStoreBox = Box<dyn TransactionStore>;
