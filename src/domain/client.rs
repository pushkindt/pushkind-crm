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
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateClient<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub phone: &'a str,
    pub address: &'a str,
    /// Updated map of custom fields.
    pub fields: HashMap<&'a str, &'a str>,
}
