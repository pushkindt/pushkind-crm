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
    /// JSON payload for the event; see SPEC.md for per-type formats.
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
    Task,
    Other(String),
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewClientEvent {
    pub client_id: ClientId,
    pub manager_id: ManagerId,
    pub event_type: ClientEventType,
    /// JSON payload for the event; see SPEC.md for per-type formats.
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
            ClientEventType::Task => write!(f, "Task"),
            ClientEventType::Other(s) => write!(f, "{s}"),
        }
    }
}

impl From<&str> for ClientEventType {
    fn from(s: &str) -> Self {
        let trimmed = s.trim();

        if trimmed.eq_ignore_ascii_case("comment") {
            ClientEventType::Comment
        } else if trimmed.eq_ignore_ascii_case("documentlink") {
            ClientEventType::DocumentLink
        } else if trimmed.eq_ignore_ascii_case("call") {
            ClientEventType::Call
        } else if trimmed.eq_ignore_ascii_case("email") {
            ClientEventType::Email
        } else if trimmed.eq_ignore_ascii_case("reply") {
            ClientEventType::Reply
        } else if trimmed.eq_ignore_ascii_case("unsubscribed") {
            ClientEventType::Unsubscribed
        } else if trimmed.eq_ignore_ascii_case("task") {
            ClientEventType::Task
        } else {
            ClientEventType::Other(s.to_string())
        }
    }
}

impl From<String> for ClientEventType {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn client_event_type_from_str_matches_known_types_case_insensitive() {
        assert_eq!(ClientEventType::from("comment"), ClientEventType::Comment);
        assert_eq!(ClientEventType::from("  CALL "), ClientEventType::Call);
        assert_eq!(
            ClientEventType::from("DocumentLink"),
            ClientEventType::DocumentLink
        );
        assert_eq!(ClientEventType::from("EMAIL"), ClientEventType::Email);
        assert_eq!(ClientEventType::from("Reply"), ClientEventType::Reply);
        assert_eq!(
            ClientEventType::from("Unsubscribed"),
            ClientEventType::Unsubscribed
        );
        assert_eq!(ClientEventType::from("task"), ClientEventType::Task);
    }

    #[test]
    fn client_event_type_from_str_preserves_original_for_other() {
        let raw = "  custom-type  ";
        assert_eq!(
            ClientEventType::from(raw),
            ClientEventType::Other(raw.to_string())
        );
    }

    #[test]
    fn client_event_type_from_string_delegates_to_str() {
        let value = "Email".to_string();
        assert_eq!(ClientEventType::from(value), ClientEventType::Email);
    }

    #[test]
    fn client_event_type_display_renders_variants() {
        assert_eq!(ClientEventType::Call.to_string(), "Call");
        assert_eq!(
            ClientEventType::Other("Custom".to_string()).to_string(),
            "Custom"
        );
    }

    #[test]
    fn new_client_event_try_new_validates_ids() {
        let event = NewClientEvent::try_new(1, 2, "comment", json!({"k": "v"}))
            .expect("expected valid ids");

        assert_eq!(event.client_id.get(), 1);
        assert_eq!(event.manager_id.get(), 2);
        assert_eq!(event.event_type, ClientEventType::Comment);
    }

    #[test]
    fn new_client_event_try_new_rejects_non_positive_ids() {
        let err = NewClientEvent::try_new(0, 1, "comment", json!({}))
            .expect_err("expected invalid client id");
        assert_eq!(err, TypeConstraintError::NonPositiveId);
    }

    #[test]
    fn client_event_try_new_constructs_event() {
        let created_at = chrono::DateTime::from_timestamp(0, 0)
            .expect("expected valid timestamp")
            .naive_utc();
        let event = ClientEvent::try_new(3, 4, 5, "reply", json!({"msg": "hi"}), created_at)
            .expect("expected valid event");

        assert_eq!(event.id.get(), 3);
        assert_eq!(event.client_id.get(), 4);
        assert_eq!(event.manager_id.get(), 5);
        assert_eq!(event.event_type, ClientEventType::Reply);
        assert_eq!(event.event_data, json!({"msg": "hi"}));
        assert_eq!(event.created_at, created_at);
    }
}
