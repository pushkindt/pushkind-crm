//! Domain model describing CRM clients.

use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::types::{
    ClientEmail, ClientId, ClientName, HubId, PhoneNumber, PublicId, TypeConstraintError,
};

/// Represent a trusted CRM client stored in the system.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Client {
    pub id: ClientId,
    pub public_id: Option<PublicId>,
    pub hub_id: HubId,
    pub name: ClientName,
    pub email: Option<ClientEmail>,
    pub phone: Option<PhoneNumber>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// Optional set of custom fields.
    pub fields: Option<BTreeMap<String, String>>,
}

impl Client {
    /// Create a trusted client from already validated domain values.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ClientId,
        public_id: Option<PublicId>,
        hub_id: HubId,
        name: ClientName,
        email: Option<ClientEmail>,
        phone: Option<PhoneNumber>,
        created_at: NaiveDateTime,
        updated_at: NaiveDateTime,
        fields: Option<BTreeMap<String, String>>,
    ) -> Self {
        Self {
            id,
            public_id,
            hub_id,
            name,
            email,
            phone,
            created_at,
            updated_at,
            fields: normalize_fields(fields),
        }
    }

    /// Create a client from raw values, validating identifiers and inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: i32,
        public_id: Option<&[u8]>,
        hub_id: i32,
        name: String,
        email: Option<String>,
        phone: Option<String>,
        created_at: NaiveDateTime,
        updated_at: NaiveDateTime,
        fields: Option<BTreeMap<String, String>>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            ClientId::try_from(id)?,
            public_id.map(PublicId::from_bytes).transpose()?,
            HubId::try_from(hub_id)?,
            ClientName::new(name)?,
            email.map(ClientEmail::try_from).transpose()?,
            phone.map(PhoneNumber::try_from).transpose()?,
            created_at,
            updated_at,
            fields,
        ))
    }
}

/// Data required to persist a new client record.
#[derive(Clone, Debug, Deserialize)]
pub struct NewClient {
    pub public_id: PublicId,
    pub hub_id: HubId,
    pub name: ClientName,
    pub email: Option<ClientEmail>,
    pub phone: Option<PhoneNumber>,
    /// Optional set of custom fields.
    pub fields: Option<BTreeMap<String, String>>,
}

impl NewClient {
    /// Create a new client from already validated domain values.
    #[must_use]
    pub fn new(
        hub_id: HubId,
        name: ClientName,
        email: Option<ClientEmail>,
        phone: Option<PhoneNumber>,
        fields: Option<BTreeMap<String, String>>,
    ) -> Self {
        Self {
            public_id: PublicId::new(),
            hub_id,
            name,
            email,
            phone,
            fields: normalize_fields(fields),
        }
    }

    /// Create a new client from raw inputs, validating identifiers and values.
    pub fn try_new(
        hub_id: i32,
        name: String,
        email: Option<String>,
        phone: Option<String>,
        fields: Option<BTreeMap<String, String>>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::try_from(hub_id)?,
            ClientName::new(name)?,
            email.map(ClientEmail::try_from).transpose()?,
            phone.map(PhoneNumber::try_from).transpose()?,
            fields,
        ))
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
    /// Create an update payload from already validated domain values.
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
            fields: normalize_fields(fields),
        }
    }

    /// Create an update payload from raw inputs, validating values.
    pub fn try_new(
        name: String,
        email: Option<String>,
        phone: Option<String>,
        fields: Option<BTreeMap<String, String>>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            ClientName::new(name)?,
            email.map(ClientEmail::try_from).transpose()?,
            phone.map(PhoneNumber::try_from).transpose()?,
            fields,
        ))
    }
}

fn normalize_fields(fields: Option<BTreeMap<String, String>>) -> Option<BTreeMap<String, String>> {
    fields.filter(|map| !map.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, str::FromStr};

    use crate::domain::types::{ClientEmail, ClientName, HubId, PhoneNumber};
    use chrono::Utc;

    fn sample_client_id() -> ClientId {
        ClientId::new(1).expect("valid client id")
    }

    fn sample_hub_id() -> HubId {
        HubId::new(1).expect("valid hub id")
    }

    fn sample_public_uuid() -> PublicId {
        PublicId::from_str("67e55044-10b1-426f-9247-bb680e5fe0c8").expect("valid public id")
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
            public_id: Some(sample_public_uuid()),
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
