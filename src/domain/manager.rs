//! Domain model for CRM hub managers.

use pushkind_common::domain::auth::AuthenticatedUser;
use serde::{Deserialize, Serialize};

use crate::domain::types::{
    ClientId, HubId, ManagerEmail, ManagerId, ManagerName, TypeConstraintError,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Manager {
    pub id: ManagerId,
    pub hub_id: HubId,
    pub name: ManagerName,
    pub email: ManagerEmail,
    pub is_user: bool,
}

impl Manager {
    /// Create a trusted manager from already validated domain values.
    #[must_use]
    pub fn new(
        id: ManagerId,
        hub_id: HubId,
        name: ManagerName,
        email: ManagerEmail,
        is_user: bool,
    ) -> Self {
        Self {
            id,
            hub_id,
            name,
            email,
            is_user,
        }
    }

    /// Create a manager from raw values, validating identifiers and inputs.
    pub fn try_new(
        id: i32,
        hub_id: i32,
        name: String,
        email: String,
        is_user: bool,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            ManagerId::try_from(id)?,
            HubId::try_from(hub_id)?,
            ManagerName::new(name)?,
            ManagerEmail::new(email)?,
            is_user,
        ))
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewManager {
    pub hub_id: HubId,
    pub name: ManagerName,
    pub email: ManagerEmail,
    pub is_user: bool,
}

impl NewManager {
    /// Create a new manager from already validated domain values.
    #[must_use]
    pub fn new(hub_id: HubId, name: ManagerName, email: ManagerEmail, is_user: bool) -> Self {
        Self {
            hub_id,
            name,
            email,
            is_user,
        }
    }

    /// Create a new manager from raw values, validating identifiers and inputs.
    pub fn try_new(
        hub_id: i32,
        name: String,
        email: String,
        is_user: bool,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::try_from(hub_id)?,
            ManagerName::new(name)?,
            ManagerEmail::new(email)?,
            is_user,
        ))
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateManager {
    pub name: ManagerName,
    pub is_user: bool,
}

impl UpdateManager {
    /// Create an update payload from already validated domain values.
    #[must_use]
    pub fn new(name: ManagerName, is_user: bool) -> Self {
        Self { name, is_user }
    }

    /// Create an update payload from raw values, validating inputs.
    pub fn try_new(name: String, is_user: bool) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(ManagerName::new(name)?, is_user))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClientManager {
    pub client_id: ClientId,
    pub manager_id: ManagerId,
}

impl ClientManager {
    /// Create a trusted client-manager link from validated identifiers.
    #[must_use]
    pub fn new(client_id: ClientId, manager_id: ManagerId) -> Self {
        Self {
            client_id,
            manager_id,
        }
    }

    /// Create a client-manager link from raw identifiers, validating IDs.
    pub fn try_new(client_id: i32, manager_id: i32) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            ClientId::try_from(client_id)?,
            ManagerId::try_from(manager_id)?,
        ))
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewClientManager {
    pub client_id: ClientId,
    pub manager_id: ManagerId,
}

impl NewClientManager {
    /// Create a new client-manager link from validated identifiers.
    #[must_use]
    pub fn new(client_id: ClientId, manager_id: ManagerId) -> Self {
        Self {
            client_id,
            manager_id,
        }
    }

    /// Create a new client-manager link from raw identifiers, validating IDs.
    pub fn try_new(client_id: i32, manager_id: i32) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            ClientId::try_from(client_id)?,
            ManagerId::try_from(manager_id)?,
        ))
    }
}

impl TryFrom<&AuthenticatedUser> for NewManager {
    type Error = TypeConstraintError;

    fn try_from(value: &AuthenticatedUser) -> Result<Self, Self::Error> {
        Ok(NewManager::new(
            HubId::try_from(value.hub_id)?,
            ManagerName::new(value.name.clone())?,
            ManagerEmail::new(value.email.clone())?,
            true,
        ))
    }
}
