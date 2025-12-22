//! Domain model that represents CRM client event history.

use std::fmt::Display;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::types::{ClientEventId, ClientId, ManagerId, TypeConstraintError};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClientEvent {
    pub id: ClientEventId,
    pub client_id: ClientId,
    pub manager_id: ManagerId,
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
    pub client_id: ClientId,
    pub manager_id: ManagerId,
    pub event_type: ClientEventType,
    pub event_data: Value,
}

impl NewClientEvent {
    /// Create a new client event from already validated domain values.
    #[must_use]
    pub fn new(
        client_id: ClientId,
        manager_id: ManagerId,
        event_type: ClientEventType,
        event_data: Value,
    ) -> Self {
        Self {
            client_id,
            manager_id,
            event_type,
            event_data,
        }
    }

    /// Create a new client event from raw identifiers, validating the IDs.
    pub fn try_new<E>(
        client_id: i32,
        manager_id: i32,
        event_type: E,
        event_data: Value,
    ) -> Result<Self, TypeConstraintError>
    where
        E: Into<ClientEventType>,
    {
        Ok(Self::new(
            ClientId::try_from(client_id)?,
            ManagerId::try_from(manager_id)?,
            event_type.into(),
            event_data,
        ))
    }
}

impl ClientEvent {
    /// Create a trusted client event from already validated domain values.
    pub fn new(
        id: ClientEventId,
        client_id: ClientId,
        manager_id: ManagerId,
        event_type: ClientEventType,
        event_data: Value,
        created_at: NaiveDateTime,
    ) -> Self {
        Self {
            id,
            client_id,
            manager_id,
            event_type,
            event_data,
            created_at,
        }
    }

    /// Create a client event from raw identifiers, validating the IDs.
    pub fn try_new<E>(
        id: i32,
        client_id: i32,
        manager_id: i32,
        event_type: E,
        event_data: Value,
        created_at: NaiveDateTime,
    ) -> Result<Self, TypeConstraintError>
    where
        E: Into<ClientEventType>,
    {
        Ok(Self::new(
            ClientEventId::try_from(id)?,
            ClientId::try_from(client_id)?,
            ManagerId::try_from(manager_id)?,
            event_type.into(),
            event_data,
            created_at,
        ))
    }
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
        match s.trim().to_lowercase().as_str() {
            "comment" => ClientEventType::Comment,
            "documentlink" => ClientEventType::DocumentLink,
            "call" => ClientEventType::Call,
            "email" => ClientEventType::Email,
            "reply" => ClientEventType::Reply,
            "unsubscribed" => ClientEventType::Unsubscribed,
            _ => ClientEventType::Other(s.to_string()),
        }
    }
}

impl From<String> for ClientEventType {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}
