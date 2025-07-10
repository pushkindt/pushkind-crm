use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::client::{
    Client as DomainClient, NewClient as DomainNewClient, UpdateClient as DomainUpdateClient,
};

#[derive(Debug, Clone, Identifiable, Queryable)]
#[diesel(table_name = crate::schema::clients)]
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

impl<'a> From<DomainNewClient<'a>> for NewClient<'a> {
    fn from(client: DomainNewClient<'a>) -> Self {
        Self {
            hub_id: client.hub_id,
            name: client.name,
            email: client.email,
            phone: client.phone,
            address: client.address,
        }
    }
}

impl<'a> From<DomainUpdateClient<'a>> for UpdateClient<'a> {
    fn from(client: DomainUpdateClient<'a>) -> Self {
        Self {
            name: client.name,
            email: client.email,
            phone: client.phone,
            address: client.address,
        }
    }
}
