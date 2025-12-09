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

#[derive(Clone, Debug, Deserialize)]
pub struct NewManager {
    pub hub_id: HubId,
    pub name: ManagerName,
    pub email: ManagerEmail,
    pub is_user: bool,
}

impl NewManager {
    #[must_use]
    pub fn new(hub_id: HubId, name: ManagerName, email: ManagerEmail, is_user: bool) -> Self {
        Self {
            hub_id,
            name,
            email,
            is_user,
        }
    }

    pub fn try_from_parts(
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClientManager {
    pub client_id: ClientId,
    pub manager_id: ManagerId,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewClientManager {
    pub client_id: ClientId,
    pub manager_id: ManagerId,
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
