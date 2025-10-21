use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use phonenumber::parse;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Client {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// Optional set of custom fields.
    pub fields: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewClient {
    pub hub_id: i32,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    /// Optional set of custom fields.
    pub fields: Option<BTreeMap<String, String>>,
}

impl NewClient {
    #[must_use]
    pub fn new(
        hub_id: i32,
        name: String,
        email: Option<String>,
        phone: Option<String>,
        fields: Option<BTreeMap<String, String>>,
    ) -> Self {
        Self {
            hub_id,
            name,
            email: email
                .map(|s| s.to_lowercase().trim().to_string())
                .filter(|s| !s.is_empty()),
            phone: normalize_phone(phone),
            fields: fields.filter(|m| !m.is_empty()),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateClient {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    /// Updated map of custom fields.
    pub fields: Option<BTreeMap<String, String>>,
}

impl UpdateClient {
    #[must_use]
    pub fn new(
        name: String,
        email: Option<String>,
        phone: Option<String>,
        fields: Option<BTreeMap<String, String>>,
    ) -> Self {
        Self {
            name,
            email: email
                .map(|s| s.to_lowercase().trim().to_string())
                .filter(|s| !s.is_empty()),
            phone: normalize_phone(phone),
            fields: fields.filter(|m| !m.is_empty()),
        }
    }
}

fn normalize_phone(phone: Option<String>) -> Option<String> {
    phone
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .and_then(|s| match parse(None, &s) {
            Ok(number) if number.is_valid() => {
                Some(format!("{}{}", number.country().code(), number.national()))
            }
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_client_normalizes_valid_phone_numbers() {
        let client = NewClient::new(
            1,
            "Alice".into(),
            None,
            Some("+1 (415) 555-2671".into()),
            None,
        );
        assert_eq!(client.phone.as_deref(), Some("14155552671"));
    }

    #[test]
    fn new_client_drops_invalid_phone_numbers() {
        let client = NewClient::new(1, "Bob".into(), None, Some("invalid".into()), None);
        assert!(client.phone.is_none());
    }

    #[test]
    fn update_client_normalizes_valid_phone_numbers() {
        let client =
            UpdateClient::new("Alice".into(), None, Some("+1 (415) 555-2671".into()), None);
        assert_eq!(client.phone.as_deref(), Some("14155552671"));
    }

    #[test]
    fn update_client_drops_invalid_phone_numbers() {
        let client = UpdateClient::new("Bob".into(), None, Some("".into()), None);
        assert!(client.phone.is_none());
    }
}
