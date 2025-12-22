//! Diesel models representing managers and tokens.

use diesel::prelude::*;

use crate::domain::manager::{
    ClientManager as DomainClientManager, Manager as DomainManager,
    NewClientManager as DomainNewClientManager, NewManager as DomainNewManager,
    UpdateManager as DomainUpdateManager,
};
use crate::domain::types::TypeConstraintError;
use crate::models::client::Client;

#[derive(Debug, Clone, Identifiable, Queryable)]
#[diesel(table_name = crate::schema::managers)]
/// Diesel model for [`crate::domain::manager::Manager`].
pub struct Manager {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: String,
    pub is_user: bool,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::managers)]
/// Insertable form of [`Manager`].
pub struct NewManager<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub email: &'a str,
    pub is_user: bool,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::managers)]
#[diesel(treat_none_as_null = true)]
/// Data used when updating a [`Manager`] record.
pub struct UpdateManager<'a> {
    pub name: &'a str,
    pub is_user: bool,
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

impl TryFrom<Manager> for DomainManager {
    type Error = TypeConstraintError;

    fn try_from(manager: Manager) -> Result<Self, Self::Error> {
        DomainManager::try_new(
            manager.id,
            manager.hub_id,
            manager.name,
            manager.email,
            manager.is_user,
        )
    }
}

impl<'a> From<&'a DomainNewManager> for NewManager<'a> {
    fn from(manager: &'a DomainNewManager) -> Self {
        Self {
            hub_id: manager.hub_id.get(),
            name: manager.name.as_str(),
            email: manager.email.as_str(),
            is_user: manager.is_user,
        }
    }
}

impl<'a> From<&'a DomainUpdateManager> for UpdateManager<'a> {
    fn from(manager: &'a DomainUpdateManager) -> Self {
        Self {
            name: manager.name.as_str(),
            is_user: manager.is_user,
        }
    }
}

impl<'a> From<&'a DomainNewManager> for UpdateManager<'a> {
    fn from(manager: &'a DomainNewManager) -> Self {
        Self {
            name: manager.name.as_str(),
            is_user: manager.is_user,
        }
    }
}

impl<'a> From<&NewManager<'a>> for UpdateManager<'a> {
    fn from(manager: &NewManager<'a>) -> Self {
        Self {
            name: manager.name,
            is_user: manager.is_user,
        }
    }
}

impl From<DomainClientManager> for ClientManager {
    fn from(manager: DomainClientManager) -> Self {
        Self {
            client_id: manager.client_id.get(),
            manager_id: manager.manager_id.get(),
        }
    }
}

impl From<DomainNewClientManager> for NewClientManager {
    fn from(manager: DomainNewClientManager) -> Self {
        Self {
            client_id: manager.client_id.get(),
            manager_id: manager.manager_id.get(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{ClientId, HubId, ManagerEmail, ManagerId, ManagerName};

    #[test]
    fn from_domain_newmanager() {
        let hub_id = HubId::new(1).expect("valid hub id");
        let name = ManagerName::new("Alice").expect("valid manager name");
        let email = ManagerEmail::new("a@b.c").expect("valid manager email");
        let domain = DomainNewManager::new(hub_id, name.clone(), email.clone(), true);
        let new: NewManager = (&domain).into();
        assert_eq!(new.hub_id, domain.hub_id.get());
        assert_eq!(new.name, domain.name.as_str());
        assert_eq!(new.email, domain.email.as_str());

        let update: UpdateManager = (&domain).into();
        assert_eq!(update.name, domain.name.as_str());

        let update_from_new: UpdateManager = (&new).into();
        assert_eq!(update_from_new.name, domain.name.as_str());
    }

    #[test]
    fn from_domain_update_manager() {
        let name = ManagerName::new("Reed").expect("valid manager name");
        let domain =
            DomainUpdateManager::try_new("Reed".to_string(), false).expect("valid update manager");
        let update: UpdateManager = (&domain).into();
        assert_eq!(update.name, name.as_str());
        assert!(!update.is_user);
    }

    #[test]
    fn from_domain_client_manager() {
        let client_id = ClientId::new(5).expect("valid client id");
        let manager_id = ManagerId::new(7).expect("valid manager id");
        let domain_client = DomainClientManager::try_new(5, 7).expect("valid client manager");
        let db_client: ClientManager = domain_client.into();
        assert_eq!(db_client.client_id, client_id.get());
        assert_eq!(db_client.manager_id, manager_id.get());
    }

    #[test]
    fn from_domain_new_client_manager() {
        let domain_new = DomainNewClientManager::try_new(9, 11).expect("valid new client manager");
        let db_new: NewClientManager = domain_new.into();
        assert_eq!(db_new.client_id, 9);
        assert_eq!(db_new.manager_id, 11);
    }

    #[test]
    fn from_manager_into_domain() {
        let db = Manager {
            id: 1,
            hub_id: 2,
            name: "Bob".into(),
            email: "b@c.d".into(),
            is_user: true,
        };
        let domain: DomainManager = DomainManager::try_from(db).expect("valid manager");
        assert_eq!(domain.id.get(), 1);
        assert_eq!(domain.hub_id.get(), 2);
        assert_eq!(domain.name.as_str(), "Bob");
        assert_eq!(domain.email.as_str(), "b@c.d");
    }
}
