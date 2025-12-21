//! Diesel models mapping CRM clients to database rows.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;

use crate::domain::client::{
    Client as DomainClient, NewClient as DomainNewClient, UpdateClient as DomainUpdateClient,
};
use crate::domain::types::{
    ClientEmail, ClientId, ClientName, HubId, PhoneNumber, TypeConstraintError,
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub fields: Option<String>,
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
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::clients)]
#[diesel(treat_none_as_null = true)]
/// Data used when updating a [`Client`] record.
pub struct UpdateClient<'a> {
    pub name: &'a str,
    pub email: Option<&'a str>,
    pub phone: Option<&'a str>,
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

impl TryFrom<Client> for DomainClient {
    type Error = TypeConstraintError;

    fn try_from(client: Client) -> Result<Self, Self::Error> {
        Ok(Self {
            id: ClientId::try_from(client.id)?,
            hub_id: HubId::try_from(client.hub_id)?,
            name: ClientName::try_from(client.name)?,
            email: client.email.map(ClientEmail::try_from).transpose()?,
            phone: client.phone.map(PhoneNumber::try_from).transpose()?,
            created_at: client.created_at,
            updated_at: client.updated_at,
            fields: None,
        })
    }
}

impl<'a> From<&'a DomainNewClient> for NewClient<'a> {
    fn from(client: &'a DomainNewClient) -> Self {
        Self {
            hub_id: client.hub_id.get(),
            name: client.name.as_str(),
            email: client.email.as_ref().map(|email| email.as_str()),
            phone: client.phone.as_ref().map(|phone| phone.as_str()),
        }
    }
}

impl<'a> From<&'a DomainUpdateClient> for UpdateClient<'a> {
    fn from(client: &'a DomainUpdateClient) -> Self {
        Self {
            name: client.name.as_str(),
            email: client.email.as_ref().map(|email| email.as_str()),
            phone: client.phone.as_ref().map(|phone| phone.as_str()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::types::{ClientEmail, ClientName, HubId, PhoneNumber};
    use chrono::Utc;

    fn sample_domain_new() -> DomainNewClient {
        DomainNewClient::new(
            HubId::new(1).expect("valid hub id"),
            ClientName::new("John").expect("valid name"),
            Some(ClientEmail::new("john@example.com").expect("valid email")),
            Some(PhoneNumber::new("+14155552671").expect("valid phone")),
            None,
        )
    }

    #[test]
    fn from_domain_new_creates_newclient() {
        let domain = sample_domain_new();
        let new: NewClient = (&domain).into();
        assert_eq!(new.hub_id, domain.hub_id.get());
        assert_eq!(new.name, domain.name.as_str());
        assert_eq!(new.email, domain.email.as_ref().map(|email| email.as_str()));
        assert_eq!(new.phone, domain.phone.as_ref().map(|phone| phone.as_str()));
    }

    #[test]
    fn from_domain_update_creates_updateclient() {
        let domain = DomainUpdateClient::new(
            ClientName::new("Jane").expect("valid name"),
            Some(ClientEmail::new("jane@example.com").expect("valid email")),
            Some(PhoneNumber::new("+14155552671").expect("valid phone")),
            Some(BTreeMap::new()),
        );
        let update: UpdateClient = (&domain).into();
        assert_eq!(update.name, domain.name.as_str());
        assert_eq!(
            update.email,
            domain.email.as_ref().map(|email| email.as_str())
        );
        assert_eq!(
            update.phone,
            domain.phone.as_ref().map(|phone| phone.as_str())
        );
    }

    #[test]
    fn client_into_domain() {
        let now: NaiveDateTime = Utc::now().naive_utc();
        let db_client = Client {
            id: 1,
            hub_id: 2,
            name: "n".to_string(),
            email: Some("e@example.com".to_string()),
            phone: Some("+14155552671".to_string()),
            created_at: now,
            updated_at: now,
            fields: None,
        };
        let domain = DomainClient::try_from(db_client).expect("valid domain client");
        assert_eq!(domain.id.get(), 1);
        assert_eq!(domain.hub_id.get(), 2);
        assert_eq!(domain.name.as_str(), "n");
        assert_eq!(domain.email.unwrap().as_str(), "e@example.com");
        assert_eq!(domain.phone.unwrap().as_str(), "+14155552671");
        assert_eq!(domain.created_at, now);
        assert_eq!(domain.updated_at, now);
    }
}
