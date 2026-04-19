//! DTOs exposed by the CRM API endpoints.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::client::Client;
use crate::domain::client_event::ClientEvent;
use crate::domain::manager::Manager;
use crate::dto::client::ClientFieldDisplay;

/// Query parameters accepted by the `/api/v1/clients` service.
#[derive(Debug, Default, Deserialize)]
pub struct ClientsQuery {
    /// Optional free-form search string applied to the client list.
    pub search: Option<String>,
    pub public_id: Option<String>,
    /// Optional page number for pagination.
    pub page: Option<usize>,
}

/// Result payload returned by [`crate::services::api::list_clients`].
#[derive(Debug)]
pub struct ClientsResponse {
    /// Total number of clients matching the filter.
    pub total: usize,
    /// Page of clients requested by the caller.
    pub clients: Vec<Client>,
}

/// A simplified client representation for React page-data APIs.
#[derive(Debug, Serialize)]
pub struct ClientListItemDto {
    pub id: i32,
    pub public_id: Option<String>,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub field_badges: Vec<String>,
}

impl From<&Client> for ClientListItemDto {
    fn from(client: &Client) -> Self {
        let field_badges = client
            .fields
            .as_ref()
            .map(|fields| fields.values().take(8).cloned().collect())
            .unwrap_or_default();

        Self {
            id: client.id.get(),
            public_id: client.public_id.as_ref().map(ToString::to_string),
            name: client.name.as_str().to_string(),
            email: client
                .email
                .as_ref()
                .map(|email| email.as_str().to_string()),
            phone: client
                .phone
                .as_ref()
                .map(|phone| phone.as_str().to_string()),
            field_badges,
        }
    }
}

/// Typed pagination state for React list pages.
#[derive(Debug, Serialize)]
pub struct PaginatedClientListDto {
    pub items: Vec<ClientListItemDto>,
    pub pages: Vec<Option<usize>>,
    pub page: usize,
}

/// Resource payload for the CRM client directory.
#[derive(Debug, Serialize)]
pub struct ClientDirectoryDto {
    pub search_query: Option<String>,
    pub clients: PaginatedClientListDto,
}

/// A simplified manager representation for React page-data APIs.
#[derive(Debug, Serialize)]
pub struct ManagerDto {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub is_user: bool,
}

impl From<&Manager> for ManagerDto {
    fn from(manager: &Manager) -> Self {
        Self {
            id: manager.id.get(),
            name: manager.name.as_str().to_string(),
            email: manager.email.as_str().to_string(),
            is_user: manager.is_user,
        }
    }
}

/// A client field display item preserved for React page rendering.
#[derive(Debug, Serialize)]
pub struct ClientFieldDisplayDto {
    pub label: String,
    pub value: Option<String>,
}

impl From<&ClientFieldDisplay> for ClientFieldDisplayDto {
    fn from(value: &ClientFieldDisplay) -> Self {
        Self {
            label: value.label.clone(),
            value: value.value.clone(),
        }
    }
}

/// Typed event representation for the client page.
#[derive(Debug, Serialize)]
pub struct ClientEventDto {
    pub id: i32,
    pub event_type: String,
    pub event_data: Value,
    pub created_at: String,
    pub manager: ManagerDto,
}

impl ClientEventDto {
    pub fn from_event_pair(event: &ClientEvent, manager: &Manager) -> Self {
        Self {
            id: event.id.get(),
            event_type: event.event_type.to_string(),
            event_data: event.event_data.clone(),
            created_at: event.created_at.to_string(),
            manager: manager.into(),
        }
    }

    pub fn from_document(event: &ClientEvent) -> Self {
        Self {
            id: event.id.get(),
            event_type: event.event_type.to_string(),
            event_data: event.event_data.clone(),
            created_at: event.created_at.to_string(),
            manager: ManagerDto {
                id: 0,
                name: String::new(),
                email: String::new(),
                is_user: false,
            },
        }
    }
}

/// Typed client details payload for React-owned client pages.
#[derive(Debug, Serialize)]
pub struct ClientDetailsDto {
    pub client: ClientDetailsHeaderDto,
    pub managers: Vec<ManagerDto>,
    pub events: Vec<ClientEventDto>,
    pub documents: Vec<ClientEventDto>,
    pub available_fields: Vec<String>,
    pub important_fields: Vec<ClientFieldDisplayDto>,
    pub other_fields: Vec<ClientFieldDisplayDto>,
    pub total_events: usize,
    pub todo_service_url: String,
    pub files_service_url: String,
}

/// Typed client header data for React-owned client pages.
#[derive(Debug, Serialize)]
pub struct ClientDetailsHeaderDto {
    pub id: i32,
    pub public_id: Option<String>,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub fields: BTreeMap<String, String>,
}

impl From<&Client> for ClientDetailsHeaderDto {
    fn from(client: &Client) -> Self {
        Self {
            id: client.id.get(),
            public_id: client.public_id.as_ref().map(ToString::to_string),
            name: client.name.as_str().to_string(),
            email: client
                .email
                .as_ref()
                .map(|email| email.as_str().to_string()),
            phone: client
                .phone
                .as_ref()
                .map(|phone| phone.as_str().to_string()),
            fields: client.fields.clone().unwrap_or_default(),
        }
    }
}

/// A managers page item combining a manager and their clients.
#[derive(Debug, Serialize)]
pub struct ManagerWithClientsDto {
    pub manager: ManagerDto,
    pub clients: Vec<ClientListItemDto>,
}

/// Typed manager collection payload for React-owned pages.
#[derive(Debug, Serialize)]
pub struct ManagerCollectionDto {
    pub managers: Vec<ManagerWithClientsDto>,
}

/// Typed manager modal payload for React-owned pages.
#[derive(Debug, Serialize)]
pub struct ManagerModalDto {
    pub manager: ManagerDto,
    pub clients: Vec<ClientListItemDto>,
}

/// Typed important-field settings payload for React-owned pages.
#[derive(Debug, Serialize)]
pub struct ImportantFieldSettingsDto {
    pub fields_text: String,
}
