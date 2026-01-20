use crate::transaction::Transaction;
use std::collections::HashMap;

pub trait TransactionStore {
    fn store(&mut self, tx: Transaction);
    fn get(&self, tx_id: u32) -> Option<&Transaction>;
}

pub struct InMemoryStore {
    transactions: HashMap<u32, Transaction>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionStore for InMemoryStore {
    fn store(&mut self, tx: Transaction) {
        self.transactions.insert(tx.tx, tx);
    }

    fn get(&self, tx_id: u32) -> Option<&Transaction> {
        self.transactions.get(&tx_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::{Transaction, TransactionType};
    use rust_decimal_macros::dec;

    #[test]
    fn test_store_and_retrieve() {
        let mut store = InMemoryStore::new();
        let tx = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 100,
            amount: Some(dec!(50.0)),
        };

        store.store(tx.clone());

        let stored = store.get(100);
        assert!(stored.is_some(), "Transaction should be found");
        assert_eq!(stored.unwrap().tx, 100);
        assert_eq!(stored.unwrap().amount, Some(dec!(50.0)));

        assert!(
            store.get(999).is_none(),
            "Non-existent transaction should return None"
        );
    }
}
