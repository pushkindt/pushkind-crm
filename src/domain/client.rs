use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Client {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// Optional set of custom fields.
    pub fields: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewClient {
    pub hub_id: i32,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    /// Optional set of custom fields.
    pub fields: Option<HashMap<String, String>>,
}

impl NewClient {
    #[must_use]
    pub fn new(
        hub_id: i32,
        name: String,
        email: Option<String>,
        phone: Option<String>,
        address: Option<String>,
        fields: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            hub_id,
            name,
            email: email
                .map(|s| s.to_lowercase().trim().to_string())
                .filter(|s| !s.is_empty()),
            phone: phone
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            address: address
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            fields: fields.filter(|m| !m.is_empty()),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateClient {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    /// Updated map of custom fields.
    pub fields: Option<HashMap<String, String>>,
}

impl UpdateClient {
    #[must_use]
    pub fn new(
        name: String,
        email: Option<String>,
        phone: Option<String>,
        address: Option<String>,
        fields: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            name,
            email: email
                .map(|s| s.to_lowercase().trim().to_string())
                .filter(|s| !s.is_empty()),
            phone: phone
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            address: address
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            fields: fields.filter(|m| !m.is_empty()),
        }
    }
}
