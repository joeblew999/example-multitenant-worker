//! Backend-agnostic store error type. Mapped to `ConnectError` at the
//! service-handler boundary so callers see proper gRPC codes (NotFound,
//! AlreadyExists, FailedPrecondition, Internal).

use std::fmt;

use connectrpc::ConnectError;

#[derive(Debug)]
pub enum StoreError {
    /// The requested row doesn't exist.
    NotFound(String),
    /// A unique constraint was violated (e.g. duplicate email signup).
    AlreadyExists(String),
    /// Domain rule violated: e.g. removing the last owner, deleting a
    /// non-empty billing account.
    Conflict(String),
    /// Underlying storage failure — D1 errored, JSON parse failed, etc.
    Backend(String),
}

impl StoreError {
    pub fn not_found(s: impl Into<String>) -> Self {
        StoreError::NotFound(s.into())
    }
    pub fn already_exists(s: impl Into<String>) -> Self {
        StoreError::AlreadyExists(s.into())
    }
    pub fn conflict(s: impl Into<String>) -> Self {
        StoreError::Conflict(s.into())
    }
    pub fn backend(s: impl Into<String>) -> Self {
        StoreError::Backend(s.into())
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NotFound(s) => write!(f, "not found: {s}"),
            StoreError::AlreadyExists(s) => write!(f, "already exists: {s}"),
            StoreError::Conflict(s) => write!(f, "conflict: {s}"),
            StoreError::Backend(s) => write!(f, "backend: {s}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<StoreError> for ConnectError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::NotFound(_) => ConnectError::not_found("resource not found"),
            StoreError::AlreadyExists(_) => ConnectError::already_exists("resource already exists"),
            StoreError::Conflict(s) => ConnectError::failed_precondition(s),
            StoreError::Backend(_) => ConnectError::internal("internal error"),
        }
    }
}

pub type StoreResult<T> = Result<T, StoreError>;
