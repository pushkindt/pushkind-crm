use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Client {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// Optional set of custom fields.
    pub fields: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewClient {
    pub hub_id: i32,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    /// Optional set of custom fields.
    pub fields: Option<HashMap<String, String>>,
}

impl NewClient {
    #[must_use]
    pub fn new(
        hub_id: i32,
        name: String,
        email: String,
        phone: String,
        address: String,
        fields: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            hub_id,
            name,
            email: email.to_lowercase(),
            phone,
            address,
            fields,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateClient {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    /// Updated map of custom fields.
    pub fields: HashMap<String, String>,
}

impl UpdateClient {
    #[must_use]
    pub fn new(
        name: String,
        email: String,
        phone: String,
        address: String,
        fields: HashMap<String, String>,
    ) -> Self {
        Self {
            name,
            email: email.to_lowercase(),
            phone,
            address,
            fields,
        }
    }
}
