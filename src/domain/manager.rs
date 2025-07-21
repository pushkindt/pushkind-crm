use pushkind_common::models::auth::AuthenticatedUser;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Manager {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewManager<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub email: &'a str,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateManager<'a> {
    pub name: &'a str,
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

impl<'a> From<&'a AuthenticatedUser> for NewManager<'a> {
    fn from(value: &'a AuthenticatedUser) -> Self {
        NewManager {
            name: &value.name,
            email: &value.email,
            hub_id: value.hub_id,
        }
    }
}
