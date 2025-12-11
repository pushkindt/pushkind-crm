//! Service adaptors serving CRM API data.

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::routes::check_role;

use crate::SERVICE_ACCESS_ROLE;
pub use crate::dto::api::{ClientsQuery, ClientsResponse};
use crate::repository::{ClientListQuery, ClientReader};
use crate::services::{ServiceError, ServiceResult};

/// Returns the filtered list of clients visible to the authenticated user.
pub fn list_clients<R>(
    repo: &R,
    user: &AuthenticatedUser,
    params: ClientsQuery,
) -> ServiceResult<ClientsResponse>
where
    R: ClientReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
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

    if let Some(search) = search {
        query = query.search(search);
    }

    let (total, clients) = repo.list_clients(query).map_err(ServiceError::from)?;

    Ok(ClientsResponse { total, clients })
}
