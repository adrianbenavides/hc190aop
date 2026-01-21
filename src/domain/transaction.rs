use crate::domain::account::Amount;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Represents the dispute state of a transaction.
///
/// - `None`: The transaction is not under dispute.
/// - `Disputed`: The transaction has been disputed and funds are held.
/// - `Chargebacked`: The dispute was finalized as a chargeback.
#[derive(Debug, PartialEq, Clone, Copy, Default, Serialize, Deserialize)]
pub enum DisputeStatus {
    #[default]
    None,
    Disputed,
    Resolved,
    Chargebacked,
}

/// The type of operation requested by a transaction.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    /// Credit to the client's account.
    Deposit,
    /// Debit from the client's account.
    Withdrawal,
    /// A claim that a transaction was erroneous.
    Dispute,
    /// A resolution to a dispute, releasing held funds.
    Resolve,
    /// A finalization of a dispute, reversing the transaction.
    Chargeback,
}

/// Represents a single financial transaction or operation.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Transaction {
    /// The type of transaction.
    pub r#type: TransactionType,
    /// The client identifier.
    pub client: u16,
    /// The global unique transaction identifier.
    pub tx: u32,
    /// The amount involved in the transaction (optional for disputes/resolves/chargebacks).
    #[serde(deserialize_with = "deserialize_optional_amount")]
    pub amount: Option<Amount>,
    /// The current dispute status of this transaction.
    #[serde(default)]
    pub dispute_status: DisputeStatus,
}

fn deserialize_optional_amount<'de, D>(deserializer: D) -> Result<Option<Amount>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val: Option<Decimal> = Option::deserialize(deserializer)?;
    match val {
        Some(d) => Amount::try_from(d)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
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
