//! Domain model describing CRM clients.

use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::types::{ClientEmail, ClientId, ClientName, HubId, PhoneNumber};

/// Represent a trusted CRM client stored in the system.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Client {
    pub id: ClientId,
    pub hub_id: HubId,
    pub name: ClientName,
    pub email: Option<ClientEmail>,
    pub phone: Option<PhoneNumber>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// Optional set of custom fields.
    pub fields: Option<BTreeMap<String, String>>,
}

/// Data required to persist a new client record.
#[derive(Clone, Debug, Deserialize)]
pub struct NewClient {
    pub hub_id: HubId,
    pub name: ClientName,
    pub email: Option<ClientEmail>,
    pub phone: Option<PhoneNumber>,
    /// Optional set of custom fields.
    pub fields: Option<BTreeMap<String, String>>,
}

impl NewClient {
    #[must_use]
    pub fn new(
        hub_id: HubId,
        name: ClientName,
        email: Option<ClientEmail>,
        phone: Option<PhoneNumber>,
        fields: Option<BTreeMap<String, String>>,
    ) -> Self {
        Self {
            hub_id,
            name,
            email,
            phone,
            fields: fields.filter(|m| !m.is_empty()),
        }
    }
}

/// Data used to update an existing client.
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateClient {
    pub name: ClientName,
    pub email: Option<ClientEmail>,
    pub phone: Option<PhoneNumber>,
    /// Updated map of custom fields.
    pub fields: Option<BTreeMap<String, String>>,
}

impl UpdateClient {
    #[must_use]
    pub fn new(
        name: ClientName,
        email: Option<ClientEmail>,
        phone: Option<PhoneNumber>,
        fields: Option<BTreeMap<String, String>>,
    ) -> Self {
        Self {
            name,
            email,
            phone,
            fields: fields.filter(|m| !m.is_empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::domain::types::{ClientEmail, ClientName, HubId, PhoneNumber};
    use chrono::Utc;

    fn sample_client_id() -> ClientId {
        ClientId::new(1).expect("valid client id")
    }

    fn sample_hub_id() -> HubId {
        HubId::new(1).expect("valid hub id")
    }

    #[test]
    fn new_client_filters_empty_field_maps() {
        let fields = Some(BTreeMap::new());
        let client = NewClient::new(
            sample_hub_id(),
            ClientName::new("Acme").expect("valid name"),
            None,
            None,
            fields,
        );
        assert!(client.fields.is_none());
    }

    #[test]
    fn update_client_filters_empty_field_maps() {
        let fields = Some(BTreeMap::new());
        let update = UpdateClient::new(
            ClientName::new("Acme").expect("valid name"),
            Some(ClientEmail::new("foo@example.com").expect("valid email")),
            Some(PhoneNumber::new("+14155552671").expect("valid phone")),
            fields,
        );
        assert!(update.fields.is_none());
    }

    #[test]
    fn client_struct_stores_typed_fields() {
        let now = Utc::now().naive_utc();
        let client = Client {
            id: sample_client_id(),
            hub_id: sample_hub_id(),
            name: ClientName::new("Test").expect("valid name"),
            email: Some(ClientEmail::new("foo@example.com").expect("valid email")),
            phone: Some(PhoneNumber::new("+14155552671").expect("valid phone")),
            created_at: now,
            updated_at: now,
            fields: None,
        };

        assert_eq!(client.id.get(), 1);
        assert_eq!(client.hub_id.get(), 1);
        assert_eq!(client.name.as_str(), "Test");
        assert_eq!(client.email.as_ref().unwrap().as_str(), "foo@example.com");
        assert_eq!(client.phone.as_ref().unwrap().as_str(), "+14155552671");
    }
}
