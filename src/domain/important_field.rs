//! Domain model for hub-specific important fields.

use serde::{Deserialize, Serialize};

/// Domain representation of a hub-specific important client field name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportantField {
    pub hub_id: i32,
    pub field: String,
}

impl ImportantField {
    #[must_use]
    pub fn new(hub_id: i32, field: String) -> Self {
        Self { hub_id, field }
    }
}
