use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use phonenumber::{Mode, country, parse};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Client {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub contact: Option<String>,
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
    pub address: Option<String>,
    pub contact: Option<String>,
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
        address: Option<String>,
        contact: Option<String>,
        fields: Option<BTreeMap<String, String>>,
    ) -> Self {
        Self {
            hub_id,
            name,
            email: normalize_email(email),
            phone: normalize_phone(phone),
            address: normalize_optional_text(address),
            contact: normalize_optional_text(contact),
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
    pub contact: Option<String>,
    /// Updated map of custom fields.
    pub fields: Option<BTreeMap<String, String>>,
}

impl UpdateClient {
    #[must_use]
    pub fn new(
        name: String,
        email: Option<String>,
        phone: Option<String>,
        address: Option<String>,
        contact: Option<String>,
        fields: Option<BTreeMap<String, String>>,
    ) -> Self {
        Self {
            name,
            email: normalize_email(email),
            phone: normalize_phone(phone),
            address: normalize_optional_text(address),
            contact: normalize_optional_text(contact),
            fields: fields.filter(|m| !m.is_empty()),
        }
    }
}

fn normalize_email(email: Option<String>) -> Option<String> {
    email
        .map(|s| s.to_lowercase().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn normalize_phone(phone: Option<String>) -> Option<String> {
    phone
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .and_then(|s| match parse(Some(country::RU), &s) {
            Ok(number) if number.is_valid() => Some(number.format().mode(Mode::E164).to_string()),
            _ => None,
        })
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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
            Some(" 1 Market St ".into()),
            Some("  ".into()),
            None,
        );
        assert_eq!(client.phone.as_deref(), Some("+14155552671"));
        assert_eq!(client.address.as_deref(), Some("1 Market St"));
        assert!(client.contact.is_none());
    }

    #[test]
    fn new_client_drops_invalid_phone_numbers() {
        let client = NewClient::new(
            1,
            "Bob".into(),
            None,
            Some("invalid".into()),
            Some(" ".into()),
            None,
            None,
        );
        assert!(client.phone.is_none());
        assert!(client.address.is_none());
    }

    #[test]
    fn update_client_normalizes_valid_phone_numbers() {
        let client = UpdateClient::new(
            "Alice".into(),
            None,
            Some("+1 (415) 555-2671".into()),
            Some(" 1 Market St".into()),
            Some("  Bob  ".into()),
            None,
        );
        assert_eq!(client.phone.as_deref(), Some("+14155552671"));
        assert_eq!(client.address.as_deref(), Some("1 Market St"));
        assert_eq!(client.contact.as_deref(), Some("Bob"));
    }

    #[test]
    fn update_client_drops_invalid_phone_numbers() {
        let client = UpdateClient::new(
            "Bob".into(),
            None,
            Some("".into()),
            Some(" ".into()),
            Some("".into()),
            None,
        );
        assert!(client.phone.is_none());
        assert!(client.address.is_none());
        assert!(client.contact.is_none());
    }
}
