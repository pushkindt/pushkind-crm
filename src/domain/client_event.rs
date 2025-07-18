use std::fmt::Display;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClientEvent {
    pub id: i32,
    pub client_id: i32,
    pub manager_id: i32,
    pub event_type: ClientEventType,
    pub event_data: Value,
    pub created_at: NaiveDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ClientEventType {
    Comment,
    DocumentLink,
    Other(String),
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewClientEvent {
    pub client_id: i32,
    pub manager_id: i32,
    pub event_type: ClientEventType,
    pub event_data: Value,
    pub created_at: NaiveDateTime,
}

impl Display for ClientEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientEventType::Comment => write!(f, "Comment"),
            ClientEventType::DocumentLink => write!(f, "DocumentLink"),
            ClientEventType::Other(s) => write!(f, "{s}"),
        }
    }
}
