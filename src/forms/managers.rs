//! Forms for creating and assigning managers.

use serde::Deserialize;
use validator::Validate;

use crate::{
    domain::{
        manager::NewManager,
        types::{ClientId, HubId, ManagerEmail, ManagerId, ManagerName},
    },
    forms::FormError,
};

#[derive(Deserialize, Validate)]
pub struct AddManagerForm {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(email)]
    pub email: String,
}

pub struct AddManagerPayload {
    pub name: ManagerName,
    pub email: ManagerEmail,
}

#[derive(Deserialize)]
pub struct AssignManagerForm {
    pub manager_id: i32,
    #[serde(default)]
    pub client_ids: Vec<i32>,
}

pub struct AssignManagerPayload {
    pub manager_id: ManagerId,
    pub client_ids: Vec<ClientId>,
}

impl TryFrom<AddManagerForm> for AddManagerPayload {
    type Error = FormError;

    fn try_from(value: AddManagerForm) -> Result<Self, Self::Error> {
        let name = ManagerName::new(value.name).map_err(|_| FormError::InvalidName)?;
        let email = ManagerEmail::try_from(value.email).map_err(|_| FormError::InvalidEmail)?;

        Ok(Self { name, email })
    }
}

impl TryFrom<AssignManagerForm> for AssignManagerPayload {
    type Error = FormError;

    fn try_from(value: AssignManagerForm) -> Result<Self, Self::Error> {
        let manager_id =
            ManagerId::new(value.manager_id).map_err(|_| FormError::InvalidManagerId)?;
        let client_ids = value
            .client_ids
            .into_iter()
            .map(|id| ClientId::new(id).map_err(|_| FormError::InvalidClientId))
            .collect::<Result<Vec<ClientId>, FormError>>()?;

        Ok(Self {
            manager_id,
            client_ids,
        })
    }
}

impl AddManagerPayload {
    pub fn into_domain(self, hub_id: HubId) -> NewManager {
        NewManager::new(hub_id, self.name, self.email, true)
    }
}
