use std::fmt;

use connectrpc::ConnectError;

#[derive(Debug)]
pub enum BillingError {
    NotFound(String),
    ProviderError(String),
}

impl BillingError {
    pub fn not_found(s: impl Into<String>) -> Self {
        BillingError::NotFound(s.into())
    }
    pub fn provider(s: impl Into<String>) -> Self {
        BillingError::ProviderError(s.into())
    }
}

impl fmt::Display for BillingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BillingError::NotFound(s) => write!(f, "not found: {s}"),
            BillingError::ProviderError(s) => write!(f, "provider error: {s}"),
        }
    }
}

impl std::error::Error for BillingError {}

impl From<BillingError> for ConnectError {
    fn from(err: BillingError) -> Self {
        match err {
            BillingError::NotFound(_) => ConnectError::not_found("resource not found"),
            BillingError::ProviderError(_) => ConnectError::internal("billing provider error"),
        }
    }
}

pub type BillingResult<T> = Result<T, BillingError>;
