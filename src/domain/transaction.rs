use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum DisputeStatus {
    #[default]
    None,
    Disputed,
    Chargebacked,
}

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
    #[serde(skip, default)]
    pub dispute_status: DisputeStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_deserialization_skips_status() {
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
        assert_eq!(result.dispute_status, DisputeStatus::None);
    }
}
