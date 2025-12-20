//! Services for the dashboard and bulk actions.

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::{check_role, ensure_role};

use crate::domain::manager::NewManager;
use crate::domain::types::HubId;
use crate::dto::main::IndexPageData;
pub use crate::dto::main::IndexQuery;
use crate::forms::main::{AddClientForm, AddClientPayload, UploadClientsForm};
use crate::repository::{ClientListQuery, ClientReader, ClientWriter, ManagerWriter};
use crate::services::{ServiceError, ServiceResult};
use crate::{SERVICE_ACCESS_ROLE, SERVICE_ADMIN_ROLE, SERVICE_MANAGER_ROLE};

/// Loads the clients list for the main index page.
pub fn load_index_page<R>(
    repo: &R,
    user: &AuthenticatedUser,
    query: IndexQuery,
) -> ServiceResult<IndexPageData>
where
    R: ClientReader + ManagerWriter + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let page = query.page.unwrap_or(1);

    let hub_id = HubId::new(user.hub_id)?;

    let mut list_query = ClientListQuery::new(hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    let search_query = query
        .search
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if let Some(search) = &search_query {
        list_query = list_query.search(search);
    }

    let (total, clients) = if check_role(SERVICE_ADMIN_ROLE, &user.roles) {
        repo.list_clients(list_query).map_err(ServiceError::from)?
    } else if check_role(SERVICE_MANAGER_ROLE, &user.roles) {
        let manager_payload = NewManager::try_from(user).map_err(|err| {
            log::error!("Failed to build manager from user: {err}");
            ServiceError::Internal
        })?;
        let manager = repo.create_or_update_manager(&manager_payload)?;
        repo.list_clients(list_query.manager_email(manager.email))
            .map_err(ServiceError::from)?
    } else {
        (0, Vec::new())
    };

    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let clients = Paginated::new(clients, page, total_pages);

    Ok(IndexPageData {
        clients,
        search_query,
    })
}

/// Validates the add-client form and persists a new client record.
pub fn add_client<R>(repo: &R, user: &AuthenticatedUser, form: AddClientForm) -> ServiceResult<()>
where
    R: ClientWriter + ?Sized,
{
    ensure_role(user, SERVICE_ADMIN_ROLE)?;

    let payload = AddClientPayload::try_from(form)?;

    let hub_id = HubId::new(user.hub_id)?;

    let new_client = payload.into_domain(hub_id);

    repo.create_clients(&[new_client])?;

    Ok(())
}

/// Parses the uploaded CSV file and creates client records in bulk.
pub fn upload_clients<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: &mut UploadClientsForm,
) -> ServiceResult<()>
where
    R: ClientWriter + ?Sized,
{
    ensure_role(user, SERVICE_ADMIN_ROLE)?;

    let hub_id = HubId::new(user.hub_id)?;

    let clients = form.parse(hub_id).map_err(|err| {
        log::error!("Failed to parse clients: {err}");
        ServiceError::Form("Ошибка при парсинге клиентов".to_string())
    })?;

    repo.create_clients(&clients)?;

    Ok(())
}
