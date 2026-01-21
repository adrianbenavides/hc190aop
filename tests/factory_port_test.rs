use hc190aop::domain::account::ClientAccount;
use hc190aop::domain::ports::{AccountStoreBox, AccountStoreFactory};
use hc190aop::infrastructure::in_memory::InMemoryAccountStore;

#[tokio::test]
async fn test_factory_instantiation() {
    let factory: AccountStoreFactory =
        Box::new(|| Box::new(InMemoryAccountStore::new()) as AccountStoreBox);

    let store = factory();
    let account = ClientAccount::new(1);

    // Verify it works
    store.store(account).await.unwrap();
    let retrieved = store.get(1).await.unwrap().unwrap();
    assert_eq!(retrieved.client, 1);
}

#[tokio::test]
async fn test_factory_in_task() {
    let factory: AccountStoreFactory =
        Box::new(|| Box::new(InMemoryAccountStore::new()) as AccountStoreBox);

    let handle = tokio::spawn(async move {
        let store = factory();
        let account = ClientAccount::new(2);
        store.store(account).await.unwrap();
        store.get(2).await.unwrap().unwrap()
    });

    let retrieved = handle.await.unwrap();
    assert_eq!(retrieved.client, 2);
}
