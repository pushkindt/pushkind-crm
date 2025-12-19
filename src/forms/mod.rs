//! Form definitions backing the CRM routes.

use thiserror::Error;
use validator::ValidationErrors;

pub mod client;
pub mod important_fields;
pub mod main;
pub mod managers;

#[derive(Debug, Error)]
/// Errors that can occur when processing form data.
pub enum FormError {
    #[error("validation errors: {0}")]
    Validation(#[from] ValidationErrors),

    #[error("invalid email address")]
    InvalidEmail,

    #[error("invalid hub_id")]
    InvalidHubId,

    #[error("invalid manager id")]
    InvalidManagerId,

    #[error("invalid client id")]
    InvalidClientId,

    #[error("invalid name")]
    InvalidName,

    #[error("invalid phone number")]
    InvalidPhoneNumber,

    #[error("invalid url")]
    InvalidUrl,
}
