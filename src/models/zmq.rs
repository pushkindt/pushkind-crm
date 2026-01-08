use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ZmqClientMessage {
    pub hub_id: i32,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    #[serde(default)]
    pub fields: Option<BTreeMap<String, String>>,
}
