use hc190aop::application::engine::PaymentEngine;
use hc190aop::domain::account::Balance;
use hc190aop::domain::transaction::{DisputeStatus, Transaction, TransactionType};
use hc190aop::infrastructure::in_memory::{InMemoryAccountStore, InMemoryTransactionStore};
use rust_decimal_macros::dec;

#[tokio::test]
async fn test_sharded_routing_correctness() {
    let af = Box::new(|| {
        Box::new(InMemoryAccountStore::new()) as hc190aop::domain::ports::AccountStoreBox
    });
    let tf = Box::new(|| {
        Box::new(InMemoryTransactionStore::new()) as hc190aop::domain::ports::TransactionStoreBox
    });

    let engine = PaymentEngine::new(af, tf);

    // Send transactions for multiple clients
    let tx1 = Transaction {
        r#type: TransactionType::Deposit,
        client: 1,
        tx: 1,
        amount: Some(dec!(100.0)),
        dispute_status: DisputeStatus::None,
    };
    let tx2 = Transaction {
        r#type: TransactionType::Deposit,
        client: 2,
        tx: 2,
        amount: Some(dec!(200.0)),
        dispute_status: DisputeStatus::None,
    };

    engine.process_transaction(tx1).await.unwrap();
    engine.process_transaction(tx2).await.unwrap();

    let results = engine.shutdown().await.unwrap();

    assert_eq!(results.len(), 2);

    let acc1 = results.iter().find(|a| a.client == 1).unwrap();
    let acc2 = results.iter().find(|a| a.client == 2).unwrap();

    assert_eq!(acc1.available, Balance(dec!(100.0)));
    assert_eq!(acc2.available, Balance(dec!(200.0)));
}
