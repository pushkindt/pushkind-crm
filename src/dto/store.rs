use serde::{Deserialize, Serialize};

use crate::domain::client::Client;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreOtpAcceptResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreSessionUser {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: Option<String>,
    pub phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreOtpVerifyResponse {
    pub success: bool,
    pub customer: StoreSessionUser,
}

impl TryFrom<Client> for StoreSessionUser {
    type Error = &'static str;

    fn try_from(value: Client) -> Result<Self, Self::Error> {
        let phone = value.phone.ok_or("client phone missing")?;

        Ok(Self {
            id: value.id.get(),
            hub_id: value.hub_id.get(),
            name: value.name.as_str().to_string(),
            email: value.email.map(|email| email.as_str().to_string()),
            phone: phone.as_str().to_string(),
        })
    }
}
