use crate::domain::transaction::Transaction;
use crate::error::{PaymentError, Result};
use std::io::Read;

/// Reads transactions from a CSV source.
///
/// This reader wraps `csv::Reader` and provides an iterator over `Result<Transaction>`.
/// It handles whitespace trimming and flexible record lengths automatically.
pub struct TransactionReader<R: Read> {
    reader: csv::Reader<R>,
}

impl<R: Read> TransactionReader<R> {
    /// Creates a new `TransactionReader` from any `Read` source (e.g., File, Stdin).
    pub fn new(source: R) -> Self {
        let reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .flexible(true)
            .from_reader(source);
        Self { reader }
    }

    /// Returns an iterator that lazily reads and deserializes transactions.
    ///
    /// This allows for processing large files in a streaming fashion without loading
    /// the entire dataset into memory.
    pub fn transactions(self) -> impl Iterator<Item = Result<Transaction>> {
        self.reader
            .into_deserialize()
            .map(|result| result.map_err(PaymentError::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_reader_valid_stream() {
        let data = "type, client, tx, amount\ndeposit, 1, 1, 1.0\nwithdrawal, 1, 2, 0.5";
        let reader = TransactionReader::new(data.as_bytes());
        let results: Vec<Result<Transaction>> = reader.transactions().collect();

        assert_eq!(results.len(), 2);
        let tx1 = results[0].as_ref().unwrap();
        assert_eq!(tx1.client, 1);
        assert_eq!(tx1.amount, Some(dec!(1.0).try_into().unwrap()));
    }

    #[test]
    fn test_reader_malformed_line() {
        let data = "type, client, tx, amount\ninvalid, 1, 1, 1.0";
        let reader = TransactionReader::new(data.as_bytes());
        let results: Vec<Result<Transaction>> = reader.transactions().collect();

        assert!(results[0].is_err());
    }
}
