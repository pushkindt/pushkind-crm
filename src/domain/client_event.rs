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
    Call,
    Email,
    Reply,
    Unsubscribed,
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
            ClientEventType::Call => write!(f, "Call"),
            ClientEventType::Email => write!(f, "Email"),
            ClientEventType::Reply => write!(f, "Reply"),
            ClientEventType::Unsubscribed => write!(f, "Unsubscribed"),
            ClientEventType::Other(s) => write!(f, "{s}"),
        }
    }
}

impl From<&str> for ClientEventType {
    fn from(s: &str) -> Self {
        match s {
            "Comment" => ClientEventType::Comment,
            "DocumentLink" => ClientEventType::DocumentLink,
            "Call" => ClientEventType::Call,
            "Email" => ClientEventType::Email,
            "Reply" => ClientEventType::Reply,
            "Unsubscribed" => ClientEventType::Unsubscribed,
            _ => ClientEventType::Other(s.to_string()),
        }
    }
}

impl From<String> for ClientEventType {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}
