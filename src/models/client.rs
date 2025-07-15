use chrono::NaiveDateTime;
use diesel::prelude::*;

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
    pub email: String,
    pub phone: String,
    pub address: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::clients)]
/// Insertable form of [`Client`].
pub struct NewClient<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub email: &'a str,
    pub phone: &'a str,
    pub address: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::clients)]
/// Data used when updating a [`Client`] record.
pub struct UpdateClient<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub phone: &'a str,
    pub address: &'a str,
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
        }
    }
}

impl<'a> From<&DomainNewClient<'a>> for NewClient<'a> {
    fn from(client: &DomainNewClient<'a>) -> Self {
        Self {
            hub_id: client.hub_id,
            name: client.name,
            email: client.email,
            phone: client.phone,
            address: client.address,
        }
    }
}

impl<'a> From<DomainNewClient<'a>> for NewClient<'a> {
    fn from(client: DomainNewClient<'a>) -> Self {
        Self::from(&client)
    }
}

impl<'a> From<&DomainUpdateClient<'a>> for UpdateClient<'a> {
    fn from(client: &DomainUpdateClient<'a>) -> Self {
        Self {
            name: client.name,
            email: client.email,
            phone: client.phone,
            address: client.address,
        }
    }
}

impl<'a> From<DomainUpdateClient<'a>> for UpdateClient<'a> {
    fn from(client: DomainUpdateClient<'a>) -> Self {
        Self::from(&client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_domain_new() -> DomainNewClient<'static> {
        DomainNewClient {
            hub_id: 1,
            name: "John",
            email: "john@example.com",
            phone: "123",
            address: "addr",
        }
    }

    #[test]
    fn from_domain_new_creates_newclient() {
        let domain = sample_domain_new();
        let new: NewClient = (&domain).into();
        assert_eq!(new.hub_id, domain.hub_id);
        assert_eq!(new.name, domain.name);
        assert_eq!(new.email, domain.email);
        assert_eq!(new.phone, domain.phone);
        assert_eq!(new.address, domain.address);
    }

    #[test]
    fn from_domain_update_creates_updateclient() {
        let domain = DomainUpdateClient {
            name: "Jane",
            email: "jane@example.com",
            phone: "321",
            address: "addr2",
        };
        let update: UpdateClient = (&domain).into();
        assert_eq!(update.name, domain.name);
        assert_eq!(update.email, domain.email);
        assert_eq!(update.phone, domain.phone);
        assert_eq!(update.address, domain.address);
    }

    #[test]
    fn client_into_domain() {
        let now: NaiveDateTime = Utc::now().naive_utc();
        let db_client = Client {
            id: 1,
            hub_id: 2,
            name: "n".into(),
            email: "e".into(),
            phone: "p".into(),
            address: "a".into(),
            created_at: now,
            updated_at: now,
        };
        let domain: DomainClient = db_client.into();
        assert_eq!(domain.id, 1);
        assert_eq!(domain.hub_id, 2);
        assert_eq!(domain.name, "n");
        assert_eq!(domain.email, "e");
        assert_eq!(domain.phone, "p");
        assert_eq!(domain.address, "a");
        assert_eq!(domain.created_at, now);
        assert_eq!(domain.updated_at, now);
    }
}
