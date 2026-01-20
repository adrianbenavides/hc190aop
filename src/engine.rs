use crate::account::ClientAccount;
use crate::error::PaymentError;
use crate::transaction::{Transaction, TransactionType};
use std::collections::HashMap;

pub struct PaymentEngine {
    pub accounts: HashMap<u16, ClientAccount>,
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
        }
    }

    pub fn process_transaction(&mut self, tx: Transaction) -> Result<(), PaymentError> {
        match tx.r#type {
            TransactionType::Deposit => self.process_deposit(tx.client, tx.amount),
            TransactionType::Withdrawal => self.process_withdrawal(tx.client, tx.amount),
            _ => Ok(()),
        }
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

        if !account.locked {
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

        if !account.locked && account.available >= amount {
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
}
