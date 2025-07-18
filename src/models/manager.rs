use diesel::prelude::*;

use crate::domain::manager::{
    ClientManager as DomainClientManager, Manager as DomainManager,
    NewClientManager as DomainNewClientManager, NewManager as DomainNewManager,
    UpdateManager as DomainUpdateManager,
};
use crate::models::client::Client;

#[derive(Debug, Clone, Identifiable, Queryable)]
#[diesel(table_name = crate::schema::managers)]
/// Diesel model for [`crate::domain::manager::Manager`].
pub struct Manager {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::managers)]
/// Insertable form of [`Manager`].
pub struct NewManager<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub email: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::managers)]
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

impl From<Manager> for DomainManager {
    fn from(manager: Manager) -> Self {
        Self {
            id: manager.id,
            hub_id: manager.hub_id,
            name: manager.name,
            email: manager.email,
        }
    }
}

impl<'a> From<&DomainNewManager<'a>> for NewManager<'a> {
    fn from(manager: &DomainNewManager<'a>) -> Self {
        Self {
            hub_id: manager.hub_id,
            name: manager.name,
            email: manager.email,
        }
    }
}

impl<'a> From<DomainNewManager<'a>> for NewManager<'a> {
    fn from(manager: DomainNewManager<'a>) -> Self {
        Self::from(&manager)
    }
}

impl<'a> From<&DomainUpdateManager<'a>> for UpdateManager<'a> {
    fn from(manager: &DomainUpdateManager<'a>) -> Self {
        Self { name: manager.name }
    }
}

impl<'a> From<DomainUpdateManager<'a>> for UpdateManager<'a> {
    fn from(manager: DomainUpdateManager<'a>) -> Self {
        Self::from(&manager)
    }
}

impl<'a> From<&NewManager<'a>> for UpdateManager<'a> {
    fn from(manager: &NewManager<'a>) -> Self {
        Self { name: manager.name }
    }
}

impl<'a> From<NewManager<'a>> for UpdateManager<'a> {
    fn from(manager: NewManager<'a>) -> Self {
        Self::from(&manager)
    }
}

impl<'a> From<&DomainNewManager<'a>> for UpdateManager<'a> {
    fn from(manager: &DomainNewManager<'a>) -> Self {
        Self { name: manager.name }
    }
}
impl<'a> From<DomainNewManager<'a>> for UpdateManager<'a> {
    fn from(manager: DomainNewManager<'a>) -> Self {
        Self::from(&manager)
    }
}

impl From<DomainClientManager> for ClientManager {
    fn from(manager: DomainClientManager) -> Self {
        Self {
            client_id: manager.client_id,
            manager_id: manager.manager_id,
        }
    }
}

impl From<DomainNewClientManager> for NewClientManager {
    fn from(manager: DomainNewClientManager) -> Self {
        Self {
            client_id: manager.client_id,
            manager_id: manager.manager_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_domain_newmanager() {
        let domain = DomainNewManager {
            hub_id: 1,
            name: "Alice",
            email: "a@b.c",
        };
        let new: NewManager = (&domain).into();
        assert_eq!(new.hub_id, domain.hub_id);
        assert_eq!(new.name, domain.name);
        assert_eq!(new.email, domain.email);

        let update: UpdateManager = (&domain).into();
        assert_eq!(update.name, domain.name);

        let update_from_new: UpdateManager = new.into();
        assert_eq!(update_from_new.name, domain.name);
    }

    #[test]
    fn from_manager_into_domain() {
        let db = Manager {
            id: 1,
            hub_id: 2,
            name: "Bob".into(),
            email: "b@c.d".into(),
        };
        let domain: DomainManager = db.into();
        assert_eq!(domain.id, 1);
        assert_eq!(domain.hub_id, 2);
        assert_eq!(domain.name, "Bob");
        assert_eq!(domain.email, "b@c.d");
    }
}
