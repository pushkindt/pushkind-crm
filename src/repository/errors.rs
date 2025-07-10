use diesel::r2d2::{Error as R2D2Error, PoolError};
use diesel::result::{DatabaseErrorKind, Error as DieselError};
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

impl From<DieselError> for RepositoryError {
    fn from(err: DieselError) -> Self {
        match err {
            DieselError::NotFound => RepositoryError::NotFound,

            DieselError::DatabaseError(kind, info) => {
                let message = info.message().to_string();
                match kind {
                    DatabaseErrorKind::UniqueViolation => RepositoryError::ConstraintViolation(
                        format!("Unique constraint violation: {message}"),
                    ),
                    DatabaseErrorKind::ForeignKeyViolation => RepositoryError::ConstraintViolation(
                        format!("Foreign key constraint violation: {message}"),
                    ),
                    DatabaseErrorKind::NotNullViolation => RepositoryError::ConstraintViolation(
                        format!("Not null constraint violation: {message}"),
                    ),
                    DatabaseErrorKind::CheckViolation => RepositoryError::ConstraintViolation(
                        format!("Check constraint violation: {message}"),
                    ),
                    _ => RepositoryError::DatabaseError(message),
                }
            }

            DieselError::InvalidCString(_) => {
                RepositoryError::ValidationError("Invalid C string".to_string())
            }

            DieselError::SerializationError(e) => {
                RepositoryError::ValidationError(format!("Serialization error: {e}"))
            }

            DieselError::DeserializationError(e) => {
                RepositoryError::ValidationError(format!("Deserialization error: {e}"))
            }

            DieselError::QueryBuilderError(e) => {
                RepositoryError::ValidationError(format!("Query builder error: {e}"))
            }

            DieselError::RollbackTransaction => {
                RepositoryError::DatabaseError("Transaction rollback".to_string())
            }

            DieselError::AlreadyInTransaction => {
                RepositoryError::DatabaseError("Already in transaction".to_string())
            }

            DieselError::NotInTransaction => {
                RepositoryError::DatabaseError("Not in transaction".to_string())
            }

            DieselError::BrokenTransactionManager => {
                RepositoryError::DatabaseError("Broken transaction manager".to_string())
            }

            _ => RepositoryError::Unexpected(format!("Unexpected diesel error: {err}")),
        }
    }
}

impl From<R2D2Error> for RepositoryError {
    fn from(err: R2D2Error) -> Self {
        RepositoryError::ConnectionError(format!("Connection error: {err}"))
    }
}

impl From<PoolError> for RepositoryError {
    fn from(err: PoolError) -> Self {
        RepositoryError::ConnectionError(format!("Connection error: {err}"))
    }
}
