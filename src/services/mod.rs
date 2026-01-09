//! Service modules defining CRM business logic.

pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

pub mod api;
pub mod client;
pub mod main;
pub mod managers;
pub mod settings;
