use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Transaction {
    pub r#type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_transaction_deserialization() {
        let csv = "type, client, tx, amount\ndeposit, 1, 1, 1.0";
        let mut reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(csv.as_bytes());
        let mut iter = reader.deserialize();

        let result: Transaction = iter
            .next()
            .unwrap()
            .expect("Failed to deserialize transaction");
        assert_eq!(result.r#type, TransactionType::Deposit);
        assert_eq!(result.client, 1);
        assert_eq!(result.tx, 1);
        assert_eq!(result.amount, Some(dec!(1.0)));
    }

    #[test]
    fn test_withdrawal_deserialization() {
        let csv = "type, client, tx, amount\nwithdrawal, 2, 5, 3.0";
        let mut reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(csv.as_bytes());
        let mut iter = reader.deserialize();

        let result: Transaction = iter.next().unwrap().unwrap();
        assert_eq!(result.r#type, TransactionType::Withdrawal);
        assert_eq!(result.client, 2);
        assert_eq!(result.tx, 5);
        assert_eq!(result.amount, Some(dec!(3.0)));
    }

    #[test]
    fn test_dispute_deserialization() {
        // Disputes don't have amounts
        let csv = "type, client, tx, amount\ndispute, 1, 1, ";
        let mut reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(csv.as_bytes());
        let mut iter = reader.deserialize();

        let result: Transaction = iter.next().unwrap().unwrap();
        assert_eq!(result.r#type, TransactionType::Dispute);
        assert_eq!(result.client, 1);
        assert_eq!(result.tx, 1);
        assert_eq!(result.amount, None);
    }
}
