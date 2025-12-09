use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use validator::Validate;

use crate::domain::client::Client;
use crate::domain::manager::NewManager;
use crate::dto::main::IndexPageData;
pub use crate::dto::main::IndexQuery;
use crate::forms::main::{AddClientForm, UploadClientsForm};
use crate::repository::{ClientListQuery, ClientReader, ClientWriter, ManagerWriter};
use crate::services::client as client_service;
use crate::services::{ServiceError, ServiceResult};
use crate::{SERVICE_ACCESS_ROLE, SERVICE_ADMIN_ROLE};

/// Loads the clients list for the main index page.
pub fn load_index_page<R>(
    repo: &R,
    user: &AuthenticatedUser,
    query: IndexQuery,
) -> ServiceResult<IndexPageData>
where
    R: ClientReader + ManagerWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let page = query.page.unwrap_or(1);
    let mut list_query = ClientListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    let search_query = query
        .search
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let (total, clients) = if let Some(term) = &search_query {
        list_query = list_query.search(term.clone());
        repo.search_clients(list_query)
            .map_err(ServiceError::from)?
    } else if check_role(SERVICE_ADMIN_ROLE, &user.roles) {
        repo.list_clients(list_query).map_err(ServiceError::from)?
    } else if check_role("crm_manager", &user.roles) {
        let manager = client_service::create_or_update_manager(repo, &NewManager::from(user))
            .map_err(|err| {
                log::error!("Failed to update manager: {err}");
                err
            })?;
        repo.list_clients(list_query.manager_email(&manager.email))
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
    if !check_role(SERVICE_ADMIN_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    if let Err(err) = form.validate() {
        log::error!("Failed to validate form: {err}");
        return Err(ServiceError::Form("Ошибка валидации формы".to_string()));
    }

    let new_client = form.to_new_client(user.hub_id);

    client_service::create_clients(repo, &[new_client]).map_err(|err| {
        log::error!("Failed to add a client: {err}");
        err
    })?;

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
    if !check_role(SERVICE_ADMIN_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let clients = form.parse(user.hub_id).map_err(|err| {
        log::error!("Failed to parse clients: {err}");
        ServiceError::Form("Ошибка при парсинге клиентов".to_string())
    })?;

    client_service::create_clients(repo, &clients).map_err(|err| {
        log::error!("Failed to add clients: {err}");
        err
    })?;

    Ok(())
}
