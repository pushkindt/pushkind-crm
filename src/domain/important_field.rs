//! Domain model for hub-specific important fields.

use serde::{Deserialize, Serialize};

use crate::domain::types::{HubId, ImportantFieldName, TypeConstraintError};

/// Domain representation of a hub-specific important client field name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportantField {
    pub hub_id: HubId,
    pub field: ImportantFieldName,
}

impl ImportantField {
    /// Create an important field from already validated domain values.
    pub fn new(hub_id: HubId, field: ImportantFieldName) -> Self {
        Self { hub_id, field }
    }

    /// Create an important field from raw values, validating identifiers and names.
    pub fn try_new(hub_id: i32, field: String) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::try_from(hub_id)?,
            ImportantFieldName::new(field)?,
        ))
    }
}
