//! DTOs exposed by the CRM API endpoints.

use crate::domain::client::Client;

/// Query parameters accepted by the `/api/v1/clients` service.
#[derive(Debug, Default)]
pub struct ClientsQuery {
    /// Optional free-form search string applied to the client list.
    pub search: Option<String>,
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
