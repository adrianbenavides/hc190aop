use hc190aop::domain::account::ClientAccount;
use hc190aop::domain::ports::{AccountStoreBox, TransactionStoreBox};
use hc190aop::domain::transaction::{Transaction, TransactionType};
use hc190aop::infrastructure::in_memory::{InMemoryAccountStore, InMemoryTransactionStore};
use rust_decimal_macros::dec;

#[tokio::test]
async fn test_stores_as_trait_objects() {
    let account_store: AccountStoreBox = Box::new(InMemoryAccountStore::new());
    let transaction_store: TransactionStoreBox = Box::new(InMemoryTransactionStore::new());

    let mut account = ClientAccount::new(1);
    account.available = hc190aop::domain::account::Balance::new(dec!(100.0));

    let tx = Transaction {
        r#type: TransactionType::Deposit,
        client: 1,
        tx: 1,
        amount: Some(dec!(100.0).try_into().unwrap()),
        dispute_status: Default::default(),
    };

    // Verify Send + Sync by spawning tasks
    let as_handle = tokio::spawn(async move {
        account_store.store(account).await.unwrap();
        account_store.get(1).await.unwrap().unwrap()
    });

    let ts_handle = tokio::spawn(async move {
        transaction_store.store(tx).await.unwrap();
        transaction_store.get(1).await.unwrap().unwrap()
    });

    let retrieved_account = as_handle.await.unwrap();
    assert_eq!(retrieved_account.client, 1);

    let retrieved_tx = ts_handle.await.unwrap();
    assert_eq!(retrieved_tx.tx, 1);
}
