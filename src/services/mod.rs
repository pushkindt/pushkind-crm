//! Service modules defining CRM business logic.

pub mod api;
pub mod client;
pub mod important_fields;
pub mod main;
pub mod managers;

pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

use crate::domain::types::TypeConstraintError;

impl From<TypeConstraintError> for ServiceError {
    fn from(_: TypeConstraintError) -> Self {
        ServiceError::Internal
    }
}
