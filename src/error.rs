use thiserror::Error;

#[derive(Error, Debug)]
pub enum PaymentError {
    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Transaction error: {0}")]
    TransactionError(String),
}
