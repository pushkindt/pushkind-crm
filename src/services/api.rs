use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::routes::check_role;

use crate::domain::client::Client;
use crate::repository::{ClientListQuery, ClientReader};
use crate::services::{ServiceError, ServiceResult};

/// Query parameters accepted by the `/api/v1/clients` service.
#[derive(Debug, Default)]
pub struct ClientsQuery {
    /// Optional free-form search string applied to the client list.
    pub search: Option<String>,
    /// Optional page number for pagination.
    pub page: Option<usize>,
}

/// Result payload returned by [`list_clients`].
#[derive(Debug)]
pub struct ClientsResponse {
    /// Total number of clients matching the filter.
    pub total: usize,
    /// Page of clients requested by the caller.
    pub clients: Vec<Client>,
}

/// Returns the filtered list of clients visible to the authenticated user.
pub fn list_clients<R>(
    repo: &R,
    user: &AuthenticatedUser,
    params: ClientsQuery,
) -> ServiceResult<ClientsResponse>
where
    R: ClientReader + ?Sized,
{
    if !check_role("crm", &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let mut query = ClientListQuery::new(user.hub_id);

    if check_role("crm_manager", &user.roles) {
        query = query.manager_email(&user.email);
    }

    if let Some(page) = params.page {
        query = query.paginate(page, DEFAULT_ITEMS_PER_PAGE);
    }

    let search = params
        .search
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let (total, clients) = if let Some(term) = search {
        repo.search_clients(query.search(term))
            .map_err(ServiceError::from)?
    } else {
        repo.list_clients(query).map_err(ServiceError::from)?
    };

    Ok(ClientsResponse { total, clients })
}
