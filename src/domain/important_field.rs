//! Domain model for hub-specific important fields.

use serde::{Deserialize, Serialize};

use crate::domain::types::{HubId, ImportantFieldName};

/// Domain representation of a hub-specific important client field name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportantField {
    pub hub_id: HubId,
    pub field: ImportantFieldName,
}

impl ImportantField {
    #[must_use]
    pub fn new(hub_id: HubId, field: ImportantFieldName) -> Self {
        Self { hub_id, field }
    }
}
