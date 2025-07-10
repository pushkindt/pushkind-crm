use diesel::prelude::*;

use crate::domain::manager::{
    ClientManager as DomainClientManager, Manager as DomainManager,
    NewClientManager as DomainNewClientManager, NewManager as DomainNewManager,
    UpdateManager as DomainUpdateManager,
};
use crate::models::client::Client;

#[derive(Debug, Clone, Identifiable, Queryable)]
#[diesel(table_name = crate::schema::clients)]
/// Diesel model for [`crate::domain::client::Manager`].
pub struct Manager {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::clients)]
/// Insertable form of [`Manager`].
pub struct NewManager<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub email: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::clients)]
/// Data used when updating a [`Manager`] record.
pub struct UpdateManager<'a> {
    pub name: &'a str,
}

#[derive(Debug, Clone, Queryable, Associations, Identifiable)]
#[diesel(primary_key(client_id, manager_id))]
#[diesel(belongs_to(Client, foreign_key=client_id))]
#[diesel(belongs_to(Manager, foreign_key=manager_id))]
#[diesel(table_name = crate::schema::client_manager)]
/// Association table linking users to roles.
pub struct ClientManager {
    pub client_id: i32,
    pub manager_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::client_manager)]
/// Insertable variant of [`ClientManager`].
pub struct NewClientManager {
    pub client_id: i32,
    pub manager_id: i32,
}

impl From<DomainManager> for Manager {
    fn from(value: DomainManager) -> Self {
        Self {
            id: value.id,
            hub_id: value.hub_id,
            name: value.name,
            email: value.email,
        }
    }
}

impl<'a> From<DomainNewManager<'a>> for NewManager<'a> {
    fn from(value: DomainNewManager<'a>) -> Self {
        Self {
            hub_id: value.hub_id,
            name: value.name,
            email: value.email,
        }
    }
}

impl<'a> From<DomainUpdateManager<'a>> for UpdateManager<'a> {
    fn from(value: DomainUpdateManager<'a>) -> Self {
        Self { name: value.name }
    }
}

impl From<DomainClientManager> for ClientManager {
    fn from(value: DomainClientManager) -> Self {
        Self {
            client_id: value.client_id,
            manager_id: value.manager_id,
        }
    }
}

impl From<DomainNewClientManager> for NewClientManager {
    fn from(value: DomainNewClientManager) -> Self {
        Self {
            client_id: value.client_id,
            manager_id: value.manager_id,
        }
    }
}
