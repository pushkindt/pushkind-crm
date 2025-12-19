//! Error conversion glue for `data` feature consumers.
//!
//! The domain layer must not depend on service/repository error types, but
//! downstream crates using `pushkind-crm` with only the `data` feature may
//! still want convenient conversions.
use pushkind_common::repository::errors::RepositoryError;
use pushkind_common::services::errors::ServiceError;

#[cfg(feature = "data")]
use crate::domain::types::TypeConstraintError;
#[cfg(feature = "server")]
use crate::forms::FormError;

#[cfg(feature = "data")]
impl From<TypeConstraintError> for ServiceError {
    fn from(val: TypeConstraintError) -> Self {
        ServiceError::TypeConstraint(val.to_string())
    }
}

#[cfg(feature = "server")]
impl From<FormError> for ServiceError {
    fn from(val: FormError) -> Self {
        ServiceError::Form(val.to_string())
    }
}

#[cfg(feature = "data")]
impl From<TypeConstraintError> for RepositoryError {
    fn from(val: TypeConstraintError) -> Self {
        RepositoryError::ValidationError(val.to_string())
    }
}
