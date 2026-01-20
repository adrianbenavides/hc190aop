use crate::account::{AccountStatus, ClientAccount};
use crate::error::PaymentError;
use crate::store::{InMemoryStore, TransactionStore};
use crate::transaction::{Transaction, TransactionType};
use std::collections::{HashMap, HashSet};

pub struct PaymentEngine<S: TransactionStore = InMemoryStore> {
    pub accounts: HashMap<u16, ClientAccount>,
    pub store: S,
    pub disputed_transactions: HashSet<u32>,
}

impl Default for PaymentEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PaymentEngine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            store: InMemoryStore::new(),
            disputed_transactions: HashSet::new(),
        }
    }
}

impl<S: TransactionStore> PaymentEngine<S> {
    pub fn with_store(store: S) -> Self {
        Self {
            accounts: HashMap::new(),
            store,
            disputed_transactions: HashSet::new(),
        }
    }

    pub fn process_transaction(&mut self, tx: Transaction) -> Result<(), PaymentError> {
        // Check for locked account first
        if let Some(account) = self.accounts.get(&tx.client) {
            if account.status == AccountStatus::Locked {
                return Ok(());
            }
        }

        match tx.r#type {
            TransactionType::Deposit => {
                self.store.store(tx.clone());
                self.process_deposit(tx.client, tx.amount)
            }
            TransactionType::Withdrawal => self.process_withdrawal(tx.client, tx.amount),
            TransactionType::Dispute => self.process_dispute(tx.client, tx.tx),
            TransactionType::Resolve => self.process_resolve(tx.client, tx.tx),
            TransactionType::Chargeback => self.process_chargeback(tx.client, tx.tx),
        }
    }

    fn process_dispute(&mut self, client: u16, tx_id: u32) -> Result<(), PaymentError> {
        if let Some(tx) = self.store.get(tx_id) {
            // Check client match and type
            if tx.client != client || tx.r#type != TransactionType::Deposit {
                return Ok(());
            }

            // Check if already disputed?
            // The requirements don't strictly forbid re-disputing, but it makes sense to only allow one active dispute per tx?
            // "Only applies to existing deposit transactions."
            // If we re-dispute, we might hold funds twice.
            // Let's prevent re-dispute.
            if self.disputed_transactions.contains(&tx_id) {
                return Ok(());
            }

            if let Some(account) = self.accounts.get_mut(&client) {
                if let Some(amount) = tx.amount {
                    account.available -= amount;
                    account.held += amount;
                    account.update_total();
                    self.disputed_transactions.insert(tx_id);
                }
            }
        }
        Ok(())
    }

    fn process_resolve(&mut self, client: u16, tx_id: u32) -> Result<(), PaymentError> {
        // Only applies if currently under dispute
        if !self.disputed_transactions.contains(&tx_id) {
            return Ok(());
        }

        if let Some(tx) = self.store.get(tx_id) {
            if tx.client != client {
                return Ok(());
            }

            if let Some(account) = self.accounts.get_mut(&client) {
                if let Some(amount) = tx.amount {
                    account.held -= amount;
                    account.available += amount;
                    account.update_total();
                    self.disputed_transactions.remove(&tx_id);
                }
            }
        }
        Ok(())
    }

    fn process_chargeback(&mut self, client: u16, tx_id: u32) -> Result<(), PaymentError> {
        // Only applies if currently under dispute
        if !self.disputed_transactions.contains(&tx_id) {
            return Ok(());
        }

        if let Some(tx) = self.store.get(tx_id) {
            if tx.client != client {
                return Ok(());
            }

            if let Some(account) = self.accounts.get_mut(&client) {
                if let Some(amount) = tx.amount {
                    account.held -= amount;
                    account.update_total();
                    account.status = AccountStatus::Locked;
                    self.disputed_transactions.remove(&tx_id);
                }
            }
        }
        Ok(())
    }

    fn process_deposit(
        &mut self,
        client: u16,
        amount: Option<rust_decimal::Decimal>,
    ) -> Result<(), PaymentError> {
        let amount = amount
            .ok_or_else(|| PaymentError::TransactionError("Deposit missing amount".to_string()))?;
        let account = self
            .accounts
            .entry(client)
            .or_insert_with(|| ClientAccount::new(client));

        if account.status == AccountStatus::Active {
            account.available += amount;
            account.update_total();
        }

        Ok(())
    }

    fn process_withdrawal(
        &mut self,
        client: u16,
        amount: Option<rust_decimal::Decimal>,
    ) -> Result<(), PaymentError> {
        let amount = amount.ok_or_else(|| {
            PaymentError::TransactionError("Withdrawal missing amount".to_string())
        })?;
        let account = self
            .accounts
            .entry(client)
            .or_insert_with(|| ClientAccount::new(client));

        if account.status == AccountStatus::Active && account.available >= amount {
            account.available -= amount;
            account.update_total();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_process_deposit() {
        let mut engine = PaymentEngine::new();
        let tx = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(10.5)),
        };

        engine.process_transaction(tx).unwrap();
        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, dec!(10.5));
        assert_eq!(account.total, dec!(10.5));
    }

    #[test]
    fn test_deposit_on_new_account() {
        let mut engine = PaymentEngine::new();
        engine.process_deposit(2, Some(dec!(5.0))).unwrap();
        let account = engine.accounts.get(&2).unwrap();
        assert_eq!(account.client, 2);
        assert_eq!(account.available, dec!(5.0));
    }

    #[test]
    fn test_process_withdrawal_sufficient_funds() {
        let mut engine = PaymentEngine::new();
        engine.process_deposit(1, Some(dec!(10.0))).unwrap();
        engine.process_withdrawal(1, Some(dec!(4.0))).unwrap();

        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, dec!(6.0));
        assert_eq!(account.total, dec!(6.0));
    }

    #[test]
    fn test_process_withdrawal_insufficient_funds() {
        let mut engine = PaymentEngine::new();
        engine.process_deposit(1, Some(dec!(10.0))).unwrap();
        engine.process_withdrawal(1, Some(dec!(11.0))).unwrap();

        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, dec!(10.0));
        assert_eq!(account.total, dec!(10.0));
    }

    #[test]
    fn test_process_dispute() {
        let mut engine = PaymentEngine::new();
        // Deposit
        let deposit = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100.0)),
        };
        engine.process_transaction(deposit).unwrap();

        // Dispute
        let dispute = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 1, // References the deposit
            amount: None,
        };
        engine.process_transaction(dispute).unwrap();

        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(100.0));
        assert_eq!(account.total, dec!(100.0));
    }

    #[test]
    fn test_process_resolve() {
        let mut engine = PaymentEngine::new();
        // Deposit 100
        engine
            .process_transaction(Transaction {
                r#type: TransactionType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(dec!(100.0)),
            })
            .unwrap();
        // Dispute 1
        engine
            .process_transaction(Transaction {
                r#type: TransactionType::Dispute,
                client: 1,
                tx: 1,
                amount: None,
            })
            .unwrap();

        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(100.0));

        // Resolve 1
        engine
            .process_transaction(Transaction {
                r#type: TransactionType::Resolve,
                client: 1,
                tx: 1,
                amount: None,
            })
            .unwrap();

        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, dec!(100.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total, dec!(100.0));

        // Resolve again should be ignored
        engine
            .process_transaction(Transaction {
                r#type: TransactionType::Resolve,
                client: 1,
                tx: 1,
                amount: None,
            })
            .unwrap();
        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, dec!(100.0)); // Unchanged
    }

    #[test]
    fn test_process_chargeback() {
        let mut engine = PaymentEngine::new();
        // Deposit 100
        engine
            .process_transaction(Transaction {
                r#type: TransactionType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(dec!(100.0)),
            })
            .unwrap();
        // Dispute 1
        engine
            .process_transaction(Transaction {
                r#type: TransactionType::Dispute,
                client: 1,
                tx: 1,
                amount: None,
            })
            .unwrap();

        // Chargeback 1
        engine
            .process_transaction(Transaction {
                r#type: TransactionType::Chargeback,
                client: 1,
                tx: 1,
                amount: None,
            })
            .unwrap();

        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total, dec!(0.0));
        assert_eq!(account.status, AccountStatus::Locked);

        // Subsquent deposit ignored
        engine
            .process_transaction(Transaction {
                r#type: TransactionType::Deposit,
                client: 1,
                tx: 2,
                amount: Some(dec!(50.0)),
            })
            .unwrap();
        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, dec!(0.0));
    }
}
