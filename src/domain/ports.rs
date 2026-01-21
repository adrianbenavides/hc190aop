use super::account::ClientAccount;
use super::transaction::Transaction;
use async_trait::async_trait;
use std::io;

#[async_trait]
pub trait AccountStore: Send + Sync {
    async fn store(&self, account: ClientAccount) -> io::Result<()>;
    async fn get(&self, client_id: u16) -> io::Result<Option<ClientAccount>>;
    async fn get_all(&self, client_id: u16) -> io::Result<Vec<ClientAccount>>;
}

#[async_trait]
pub trait TransactionStore: Send + Sync {
    async fn store(&self, tx: Transaction) -> io::Result<()>;
    async fn get(&self, tx_id: u32) -> io::Result<Option<Transaction>>;
}

pub type AccountStoreBox = Box<dyn AccountStore>;
pub type TransactionStoreBox = Box<dyn TransactionStore>;

pub type AccountStoreFactory = Box<dyn Fn() -> AccountStoreBox + Send + Sync>;
pub type TransactionStoreFactory = Box<dyn Fn() -> TransactionStoreBox + Send + Sync>;
