//! Services for the dashboard and bulk actions.

use std::str::FromStr;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::{check_role, ensure_role};

use crate::domain::manager::NewManager;
use crate::domain::types::{HubId, PublicId};
use crate::dto::main::IndexPageData;
pub use crate::dto::main::IndexQuery;
use crate::forms::main::{AddClientForm, AddClientPayload, UploadClientsForm};
use crate::repository::{ClientListQuery, ClientReader, ClientWriter, ManagerWriter};
use crate::services::{ServiceError, ServiceResult};
use crate::{SERVICE_ACCESS_ROLE, SERVICE_ADMIN_ROLE, SERVICE_MANAGER_ROLE};

/// Loads the clients list for the main index page.
pub fn load_index_page<R>(
    query: IndexQuery,
    user: &AuthenticatedUser,
    repo: &R,
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
    if let Some(public_id_raw) = query
        .public_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        match PublicId::from_str(public_id_raw) {
            Ok(public_id) => {
                list_query = list_query.public_id(public_id);
            }
            Err(_) => {
                return Ok(IndexPageData {
                    clients: Paginated::new(Vec::new(), page, 0),
                    search_query,
                });
            }
        }
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
pub fn add_client<R>(form: AddClientForm, user: &AuthenticatedUser, repo: &R) -> ServiceResult<()>
where
    R: ClientWriter + ?Sized,
{
    ensure_role(user, SERVICE_ADMIN_ROLE)?;

    let payload = AddClientPayload::try_from(form)?;

    let hub_id = HubId::new(user.hub_id)?;

    let new_client = payload.into_domain(hub_id);

    repo.create_or_replace_clients(&[new_client])?;

    Ok(())
}

/// Parses the uploaded CSV file and creates client records in bulk.
pub fn upload_clients<R>(
    form: &mut UploadClientsForm,
    user: &AuthenticatedUser,
    repo: &R,
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

    repo.create_or_replace_clients(&clients)?;

    Ok(())
}

#[cfg(all(test, feature = "test-mocks"))]
mod tests {
    use super::*;
    use crate::domain::client::Client;
    use crate::domain::manager::Manager;
    use crate::domain::types::{ClientName, HubId, ManagerEmail, ManagerName, PublicId};
    use crate::repository::mock::MockRepository;
    use chrono::Utc;
    use pushkind_common::services::errors::ServiceError;

    fn access_user() -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "1".to_string(),
            email: "viewer@example.com".to_string(),
            hub_id: 11,
            name: "Viewer".to_string(),
            roles: vec![SERVICE_ACCESS_ROLE.to_string()],
            exp: 0,
        }
    }

    fn admin_user() -> AuthenticatedUser {
        let mut user = access_user();
        user.roles.push(SERVICE_ADMIN_ROLE.to_string());
        user
    }

    fn manager_user() -> AuthenticatedUser {
        let mut user = access_user();
        user.roles.push(SERVICE_MANAGER_ROLE.to_string());
        user
    }

    fn sample_client(id: i32, hub_id: i32) -> Client {
        Client::try_new(
            id,
            Some(PublicId::new().as_bytes()),
            hub_id,
            "Client".to_string(),
            Some("client@example.com".to_string()),
            None,
            Utc::now().naive_utc(),
            Utc::now().naive_utc(),
            None,
        )
        .expect("valid client")
    }

    fn sample_manager(id: i32, hub_id: i32) -> Manager {
        Manager::try_new(
            id,
            hub_id,
            "Manager".to_string(),
            "manager@example.com".to_string(),
            true,
        )
        .expect("valid manager")
    }

    #[test]
    fn load_index_page_requires_access_role() {
        let mut repo = MockRepository::new();
        repo.expect_list_clients().times(0);
        repo.expect_create_or_update_manager().times(0);
        let mut user = access_user();
        user.roles.clear();

        let result = load_index_page(IndexQuery::default(), &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_index_page_for_admin_applies_search() {
        let mut repo = MockRepository::new();
        repo.expect_create_or_update_manager().times(0);
        let expected_client = sample_client(1, 11);
        repo.expect_list_clients()
            .withf(|query| {
                query.hub_id == HubId::new(11).expect("valid hub id")
                    && query.manager_email.is_none()
                    && query.search.as_deref() == Some("Delta")
                    && query.pagination.as_ref().is_some_and(|pagination| {
                        pagination.page == 2 && pagination.per_page == DEFAULT_ITEMS_PER_PAGE
                    })
            })
            .times(1)
            .returning(move |_| Ok((1, vec![expected_client.clone()])));

        let user = admin_user();
        let query = IndexQuery {
            search: Some("  Delta  ".to_string()),
            page: Some(2),
            public_id: None,
        };

        let data = load_index_page(query, &user, &repo).expect("page data");

        assert_eq!(data.search_query, Some("Delta".to_string()));
    }

    #[test]
    fn load_index_page_for_manager_scopes_clients() {
        let mut repo = MockRepository::new();
        let manager = sample_manager(3, 11);
        let manager_email = manager.email.clone();
        repo.expect_create_or_update_manager()
            .withf(|payload| {
                payload.hub_id == HubId::new(11).expect("valid hub id")
                    && payload.is_user
                    && payload.email == ManagerEmail::new("viewer@example.com").expect("email")
                    && payload.name == ManagerName::new("Viewer").expect("name")
            })
            .times(1)
            .returning(move |_| Ok(manager.clone()));

        let expected_client = sample_client(2, 11);
        repo.expect_list_clients()
            .withf(move |query| {
                query.hub_id == HubId::new(11).expect("valid hub id")
                    && query.manager_email.as_ref() == Some(&manager_email)
                    && query.search.is_none()
                    && query.pagination.as_ref().is_some_and(|pagination| {
                        pagination.page == 1 && pagination.per_page == DEFAULT_ITEMS_PER_PAGE
                    })
            })
            .times(1)
            .returning(move |_| Ok((1, vec![expected_client.clone()])));

        let user = manager_user();
        let data = load_index_page(IndexQuery::default(), &user, &repo).expect("page data");

        assert_eq!(data.search_query, None);
    }

    #[test]
    fn load_index_page_for_viewer_returns_empty() {
        let mut repo = MockRepository::new();
        repo.expect_list_clients().times(0);
        repo.expect_create_or_update_manager().times(0);
        let user = access_user();

        let data = load_index_page(IndexQuery::default(), &user, &repo).expect("page data");

        assert_eq!(data.search_query, None);
    }

    #[test]
    fn add_client_persists_new_client() {
        let mut repo = MockRepository::new();
        repo.expect_create_or_replace_clients()
            .withf(|clients| {
                clients.len() == 1
                    && clients[0].hub_id == HubId::new(11).expect("valid hub id")
                    && clients[0].name == ClientName::new("Alice").expect("name")
            })
            .times(1)
            .returning(|_| Ok(1));

        let user = admin_user();
        let form = AddClientForm {
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
            phone: None,
        };

        add_client(form, &user, &repo).expect("client created");
    }

    #[test]
    fn load_index_page_with_invalid_public_id_returns_empty_without_repo_query() {
        let mut repo = MockRepository::new();
        repo.expect_list_clients().times(0);
        repo.expect_create_or_update_manager().times(0);

        let user = admin_user();
        let query = IndexQuery {
            public_id: Some("not-a-uuid".to_string()),
            ..Default::default()
        };

        let data = load_index_page(query, &user, &repo).expect("page data");

        let clients_value = serde_json::to_value(&data.clients).expect("serialize clients");
        let items = clients_value
            .get("items")
            .and_then(|value| value.as_array())
            .expect("items array");
        assert!(items.is_empty());
    }
}
