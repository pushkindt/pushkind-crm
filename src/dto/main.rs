use pushkind_common::pagination::Paginated;

use crate::domain::client::Client;

/// Query parameters accepted by the index page service.
#[derive(Debug, Default)]
pub struct IndexQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
    /// Page number requested by the user interface.
    pub page: Option<usize>,
}

/// Data required to render the main index template.
pub struct IndexPageData {
    /// Paginated list of clients to show in the table.
    pub clients: Paginated<Client>,
    /// Search query echoed back to the template when present.
    pub search_query: Option<String>,
}
