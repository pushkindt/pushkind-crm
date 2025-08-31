use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;

use crate::domain::client::{
    Client as DomainClient, NewClient as DomainNewClient, UpdateClient as DomainUpdateClient,
};

#[derive(Debug, Clone, Identifiable, Queryable, QueryableByName)]
#[diesel(table_name = crate::schema::clients)]
#[diesel(foreign_derive)]
/// Diesel model for [`crate::domain::client::Client`].
pub struct Client {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(QueryableByName)]
pub struct ClientCount {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub count: i64,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::clients)]
/// Insertable form of [`Client`].
pub struct NewClient<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub email: Option<&'a str>,
    pub phone: Option<&'a str>,
    pub address: Option<&'a str>,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::clients)]
/// Data used when updating a [`Client`] record.
pub struct UpdateClient<'a> {
    pub name: &'a str,
    pub email: Option<&'a str>,
    pub phone: Option<&'a str>,
    pub address: Option<&'a str>,
}

#[derive(Identifiable, Queryable, Selectable, Associations, Insertable, Serialize)]
#[diesel(table_name = crate::schema::client_fields)]
#[diesel(belongs_to(Client, foreign_key = client_id))]
#[diesel(primary_key(client_id, field))]
pub struct ClientField {
    pub client_id: i32,
    pub field: String,
    pub value: String,
}

impl From<Client> for DomainClient {
    fn from(client: Client) -> Self {
        Self {
            id: client.id,
            hub_id: client.hub_id,
            name: client.name,
            email: client.email,
            phone: client.phone,
            address: client.address,
            created_at: client.created_at,
            updated_at: client.updated_at,
            fields: None,
        }
    }
}

impl<'a> From<&'a DomainNewClient> for NewClient<'a> {
    fn from(client: &'a DomainNewClient) -> Self {
        Self {
            hub_id: client.hub_id,
            name: client.name.as_str(),
            email: client.email.as_deref(),
            phone: client.phone.as_deref(),
            address: client.address.as_deref(),
        }
    }
}

impl<'a> From<&'a DomainUpdateClient> for UpdateClient<'a> {
    fn from(client: &'a DomainUpdateClient) -> Self {
        Self {
            name: client.name.as_str(),
            email: client.email.as_deref(),
            phone: client.phone.as_deref(),
            address: client.address.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use chrono::Utc;

    fn sample_domain_new() -> DomainNewClient {
        DomainNewClient::new(
            1,
            "John".to_string(),
            Some("john@example.com".to_string()),
            Some("123".to_string()),
            Some("addr".to_string()),
            None,
        )
    }

    #[test]
    fn from_domain_new_creates_newclient() {
        let domain = sample_domain_new();
        let new: NewClient = (&domain).into();
        assert_eq!(new.hub_id, domain.hub_id);
        assert_eq!(new.name, domain.name);
        assert_eq!(new.email, domain.email.as_deref());
        assert_eq!(new.phone, domain.phone.as_deref());
        assert_eq!(new.address, domain.address.as_deref());
    }

    #[test]
    fn from_domain_update_creates_updateclient() {
        let domain = DomainUpdateClient::new(
            "Jane".to_string(),
            Some("jane@example.com".to_string()),
            Some("321".to_string()),
            Some("addr2".to_string()),
            Some(HashMap::new()),
        );
        let update: UpdateClient = (&domain).into();
        assert_eq!(update.name, domain.name);
        assert_eq!(update.email, domain.email.as_deref());
        assert_eq!(update.phone, domain.phone.as_deref());
        assert_eq!(update.address, domain.address.as_deref());
    }

    #[test]
    fn client_into_domain() {
        let now: NaiveDateTime = Utc::now().naive_utc();
        let db_client = Client {
            id: 1,
            hub_id: 2,
            name: "n".to_string(),
            email: Some("e".to_string()),
            phone: Some("p".to_string()),
            address: Some("a".to_string()),
            created_at: now,
            updated_at: now,
        };
        let domain: DomainClient = db_client.into();
        assert_eq!(domain.id, 1);
        assert_eq!(domain.hub_id, 2);
        assert_eq!(domain.name, "n");
        assert_eq!(domain.email, Some("e".to_string()));
        assert_eq!(domain.phone, Some("p".to_string()));
        assert_eq!(domain.address, Some("a".to_string()));
        assert_eq!(domain.created_at, now);
        assert_eq!(domain.updated_at, now);
    }
}
