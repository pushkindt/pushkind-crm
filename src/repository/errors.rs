use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Entity not found")]
    NotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;
