use miette::Diagnostic;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, PaymentError>;

#[derive(Error, Debug, Diagnostic)]
pub enum PaymentError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    InternalError(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<csv::Error> for PaymentError {
    fn from(err: csv::Error) -> Self {
        PaymentError::InternalError(Box::new(err))
    }
}

impl From<std::io::Error> for PaymentError {
    fn from(err: std::io::Error) -> Self {
        PaymentError::InternalError(Box::new(err))
    }
}

impl From<tokio::task::JoinError> for PaymentError {
    fn from(err: tokio::task::JoinError) -> Self {
        PaymentError::InternalError(Box::new(err))
    }
}

impl From<rocksdb::Error> for PaymentError {
    fn from(err: rocksdb::Error) -> Self {
        PaymentError::InternalError(Box::new(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversions() {
        let io_err = std::io::Error::other("test io error");
        let payment_err: PaymentError = io_err.into();
        // Check it maps to InternalError
        assert!(matches!(payment_err, PaymentError::InternalError(_)));
        // Check the string representation contains the inner error message
        assert!(payment_err.to_string().contains("test io error"));
    }
}
