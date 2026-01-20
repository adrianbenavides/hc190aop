use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum PaymentError {
    #[error("CSV processing error")]
    #[diagnostic(code(payment::csv_error), help("Ensure the CSV format is correct"))]
    CsvError(#[from] csv::Error),

    #[error("I/O error")]
    #[diagnostic(code(payment::io_error))]
    IoError(#[from] std::io::Error),

    #[error("Transaction error: {0}")]
    #[diagnostic(code(payment::transaction_error))]
    TransactionError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PaymentError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        assert_eq!(err.to_string(), "I/O error");
    }
}
