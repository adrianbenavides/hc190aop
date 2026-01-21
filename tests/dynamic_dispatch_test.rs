use hc190aop::domain::account::ClientAccount;
use hc190aop::domain::ports::{AccountStore, TransactionStore};
use hc190aop::domain::transaction::{Transaction, TransactionType};
use hc190aop::infrastructure::in_memory::{InMemoryAccountStore, InMemoryTransactionStore};
use rust_decimal_macros::dec;
use std::sync::Arc;

#[tokio::test]
async fn test_stores_as_trait_objects() {
    let account_store: Arc<dyn AccountStore> = Arc::new(InMemoryAccountStore::new());
    let transaction_store: Arc<dyn TransactionStore> = Arc::new(InMemoryTransactionStore::new());

    let mut account = ClientAccount::new(1);
    account.available = hc190aop::domain::account::Balance::new(dec!(100.0));

    let tx = Transaction {
        r#type: TransactionType::Deposit,
        client: 1,
        tx: 1,
        amount: Some(dec!(100.0)),
        dispute_status: Default::default(),
    };

    // Verify Send + Sync by spawning tasks
    let as_clone = account_store.clone();
    let as_handle = tokio::spawn(async move {
        as_clone.store(account).await.unwrap();
    });

    let ts_clone = transaction_store.clone();
    let ts_handle = tokio::spawn(async move {
        ts_clone.store(tx).await.unwrap();
    });

    as_handle.await.unwrap();
    ts_handle.await.unwrap();

    let retrieved_account = account_store.get(1).await.unwrap().unwrap();
    assert_eq!(retrieved_account.client, 1);

    let retrieved_tx = transaction_store.get(1).await.unwrap().unwrap();
    assert_eq!(retrieved_tx.tx, 1);
}
