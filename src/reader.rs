use crate::error::PaymentError;
use crate::transaction::Transaction;
use std::io::Read;

pub struct TransactionReader<R: Read> {
    reader: csv::Reader<R>,
}

impl<R: Read> TransactionReader<R> {
    pub fn new(source: R) -> Self {
        let reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .flexible(true)
            .from_reader(source);
        Self { reader }
    }

    pub fn transactions(self) -> impl Iterator<Item = Result<Transaction, PaymentError>> {
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
        let results: Vec<Result<Transaction, PaymentError>> = reader.transactions().collect();

        assert_eq!(results.len(), 2);
        let tx1 = results[0].as_ref().unwrap();
        assert_eq!(tx1.client, 1);
        assert_eq!(tx1.amount, Some(dec!(1.0)));
    }

    #[test]
    fn test_reader_malformed_line() {
        let data = "type, client, tx, amount\ninvalid, 1, 1, 1.0";
        let reader = TransactionReader::new(data.as_bytes());
        let results: Vec<Result<Transaction, PaymentError>> = reader.transactions().collect();

        assert!(results[0].is_err());
    }
}
