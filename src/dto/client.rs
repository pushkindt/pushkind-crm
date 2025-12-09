//! DTOs shaped for client detail and edit templates.

use serde::Serialize;

use crate::domain::client::Client;
use crate::domain::client_event::ClientEvent;
use crate::domain::manager::Manager;
use crate::domain::types::ClientId;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ClientFieldDisplay {
    pub label: String,
    pub value: Option<String>,
}

/// Aggregated data required to render the client details page.
#[derive(Debug)]
pub struct ClientPageData {
    pub client: Client,
    pub managers: Vec<Manager>,
    pub events_with_managers: Vec<(ClientEvent, Manager)>,
    pub documents: Vec<ClientEvent>,
    pub available_fields: Vec<String>,
    pub important_fields: Vec<ClientFieldDisplay>,
    pub other_fields: Vec<ClientFieldDisplay>,
    pub total_events: usize,
}

/// Generic result wrapper for client mutations so callers can redirect easily.
#[derive(Debug)]
pub struct ClientOperationOutcome {
    pub client_id: ClientId,
}
