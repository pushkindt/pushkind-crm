use pushkind_common::domain::auth::AuthenticatedUser;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Manager {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewManager {
    pub hub_id: i32,
    pub name: String,
    pub email: String,
}

impl NewManager {
    #[must_use]
    pub fn new(hub_id: i32, name: String, email: String) -> Self {
        Self {
            hub_id,
            name,
            email: email.to_lowercase(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateManager {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClientManager {
    pub client_id: i32,
    pub manager_id: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewClientManager {
    pub client_id: i32,
    pub manager_id: i32,
}

impl From<&AuthenticatedUser> for NewManager {
    fn from(value: &AuthenticatedUser) -> Self {
        NewManager::new(value.hub_id, value.name.clone(), value.email.clone())
    }
}
