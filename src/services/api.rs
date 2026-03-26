//! Service adaptors serving CRM API data.

use std::str::FromStr;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::routes::check_role;
use serde::Deserialize;

use crate::domain::types::{HubId, PublicId};
use crate::dto::api::{
    ClientDetailsDto, ClientDetailsHeaderDto, ClientEventDto, ClientFieldDisplayDto,
    ClientListItemDto, DashboardPageDto, ManagerModalDto, ManagerWithClientsDto, ManagersPageDto,
    NoAccessPageDto, PaginatedClientListDto, SettingsPageDto,
};
pub use crate::dto::api::{ClientsQuery, ClientsResponse, IamDto, NavigationItemDto};
use crate::models::config::AppConfig;
use crate::repository::{ClientListQuery, ClientReader};
use crate::services::{ServiceError, ServiceResult, client, main, managers, settings};
use crate::{SERVICE_ACCESS_ROLE, SERVICE_ADMIN_ROLE};

#[derive(Debug, Deserialize)]
struct SerializedPaginated<T> {
    items: Vec<T>,
    pages: Vec<Option<usize>>,
    page: usize,
}

fn has_shell_access(user: &AuthenticatedUser) -> bool {
    check_role(SERVICE_ACCESS_ROLE, &user.roles) || check_role(SERVICE_ADMIN_ROLE, &user.roles)
}

/// Returns typed shell data for React-owned CRM pages.
pub fn get_shell_data(
    user: &AuthenticatedUser,
    common_config: &CommonServerConfig,
) -> ServiceResult<IamDto> {
    if !has_shell_access(user) {
        return Err(ServiceError::Unauthorized);
    }

    let has_crm_access = check_role(SERVICE_ACCESS_ROLE, &user.roles);
    let is_admin = check_role(SERVICE_ADMIN_ROLE, &user.roles);

    let mut navigation = Vec::new();
    let mut local_menu_items = Vec::new();

    if has_crm_access {
        navigation.push(NavigationItemDto {
            name: "Клиенты",
            url: "/",
        });
    }

    if is_admin {
        navigation.push(NavigationItemDto {
            name: "Менеджеры",
            url: "/managers",
        });
        local_menu_items.push(NavigationItemDto {
            name: "Настройки",
            url: "/settings",
        });
    }

    Ok(IamDto {
        current_user: user.into(),
        home_url: common_config.auth_service_url.clone(),
        navigation,
        local_menu_items,
    })
}

/// Returns minimal page data for the local no-access page.
pub fn get_no_access_data(
    user: &AuthenticatedUser,
    common_config: &CommonServerConfig,
) -> NoAccessPageDto {
    NoAccessPageDto {
        current_user: user.into(),
        home_url: common_config.auth_service_url.clone(),
    }
}

/// Returns the filtered list of clients visible to the authenticated user.
pub fn list_clients<R>(
    params: ClientsQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ClientsResponse>
where
    R: ClientReader + ?Sized,
{
    if !has_shell_access(user) {
        return Err(ServiceError::Unauthorized);
    }

    let mut query = ClientListQuery::new(HubId::new(user.hub_id)?);

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
    if let Some(public_id_raw) = params
        .public_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        match PublicId::from_str(public_id_raw) {
            Ok(public_id) => {
                query = query.public_id(public_id);
            }
            Err(_) => {
                return Ok(ClientsResponse {
                    total: 0,
                    clients: Vec::new(),
                });
            }
        }
    }

    let (total, clients) = repo.list_clients(query).map_err(ServiceError::from)?;

    Ok(ClientsResponse { total, clients })
}

/// Returns typed page data for the CRM dashboard.
pub fn get_dashboard_data<R>(
    params: main::IndexQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<DashboardPageDto>
where
    R: crate::repository::ClientReader + crate::repository::ManagerWriter + ?Sized,
{
    let can_add_client = check_role(SERVICE_ADMIN_ROLE, &user.roles);
    let data = main::load_index_page(params, user, repo)?;
    let paginated_clients: SerializedPaginated<crate::domain::client::Client> =
        serde_json::from_value(
            serde_json::to_value(data.clients).map_err(|_| ServiceError::Internal)?,
        )
        .map_err(|_| ServiceError::Internal)?;

    Ok(DashboardPageDto {
        search_query: data.search_query,
        clients: PaginatedClientListDto {
            items: paginated_clients
                .items
                .iter()
                .map(ClientListItemDto::from)
                .collect(),
            pages: paginated_clients.pages,
            page: paginated_clients.page,
        },
        can_add_client,
    })
}

/// Returns typed page data for the CRM client details page.
pub fn get_client_details_data<R>(
    client_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
    app_config: &AppConfig,
) -> ServiceResult<ClientDetailsDto>
where
    R: crate::repository::ClientReader
        + crate::repository::ClientEventReader
        + crate::repository::ImportantFieldReader
        + ?Sized,
{
    let data = client::load_client_details(client_id, user, repo)?;

    Ok(ClientDetailsDto {
        client: ClientDetailsHeaderDto::from(&data.client),
        managers: data.managers.iter().map(Into::into).collect(),
        events: data
            .events_with_managers
            .iter()
            .map(|(event, manager)| ClientEventDto::from_event_pair(event, manager))
            .collect(),
        documents: data
            .documents
            .iter()
            .map(ClientEventDto::from_document)
            .collect(),
        available_fields: data.available_fields,
        important_fields: data
            .important_fields
            .iter()
            .map(Into::into)
            .collect::<Vec<ClientFieldDisplayDto>>(),
        other_fields: data
            .other_fields
            .iter()
            .map(Into::into)
            .collect::<Vec<ClientFieldDisplayDto>>(),
        total_events: data.total_events,
        todo_service_url: app_config.todo_service_url.clone(),
        files_service_url: app_config.files_service_url.clone(),
    })
}

/// Returns typed page data for the CRM managers page.
pub fn get_managers_page_data<R>(
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ManagersPageDto>
where
    R: crate::repository::ManagerReader + ?Sized,
{
    let data = managers::list_managers(user, repo)?;

    Ok(ManagersPageDto {
        managers: data
            .managers
            .iter()
            .map(|(manager, clients)| ManagerWithClientsDto {
                manager: manager.into(),
                clients: clients.iter().map(ClientListItemDto::from).collect(),
            })
            .collect(),
    })
}

/// Returns typed page data for the manager modal.
pub fn get_manager_modal_data<R>(
    manager_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ManagerModalDto>
where
    R: crate::repository::ManagerReader + crate::repository::ClientReader + ?Sized,
{
    let data = managers::load_manager_modal(manager_id, user, repo)?;

    Ok(ManagerModalDto {
        manager: (&data.manager).into(),
        clients: data.clients.iter().map(ClientListItemDto::from).collect(),
    })
}

/// Returns typed page data for the settings page.
pub fn get_settings_page_data<R>(
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<SettingsPageDto>
where
    R: crate::repository::ImportantFieldReader + ?Sized,
{
    let data = settings::load_important_fields(user, repo)?;

    Ok(SettingsPageDto {
        fields_text: data.fields.join("\n"),
    })
}

#[cfg(all(test, feature = "test-mocks"))]
mod tests {
    use super::*;
    use crate::domain::client::Client;
    use crate::domain::types::{ClientId, ClientName, HubId, PublicId};
    use crate::repository::mock::MockRepository;
    use crate::services::ServiceError;
    use chrono::Utc;

    fn access_user() -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "1".to_string(),
            email: "viewer@example.com".to_string(),
            hub_id: 7,
            name: "Viewer".to_string(),
            roles: vec![SERVICE_ACCESS_ROLE.to_string()],
            exp: 0,
        }
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

    #[test]
    fn get_shell_data_requires_access_role() {
        let mut user = access_user();
        user.roles.clear();

        let result = get_shell_data(
            &user,
            &CommonServerConfig {
                auth_service_url: "https://auth.example.com".to_string(),
                secret: "secret".to_string(),
            },
        );

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn get_shell_data_includes_admin_navigation() {
        let mut user = access_user();
        user.roles.push(SERVICE_ADMIN_ROLE.to_string());

        let response = get_shell_data(
            &user,
            &CommonServerConfig {
                auth_service_url: "https://auth.example.com".to_string(),
                secret: "secret".to_string(),
            },
        )
        .expect("shell data");

        assert_eq!(response.current_user.email, "viewer@example.com");
        assert_eq!(response.home_url, "https://auth.example.com");
        assert!(response.navigation.iter().any(|item| item.url == "/"));
        assert!(
            response
                .navigation
                .iter()
                .any(|item| item.url == "/managers")
        );
        assert!(
            response
                .local_menu_items
                .iter()
                .any(|item| item.url == "/settings")
        );
    }

    #[test]
    fn get_shell_data_allows_admin_only_users() {
        let mut user = access_user();
        user.roles = vec![SERVICE_ADMIN_ROLE.to_string()];

        let response = get_shell_data(
            &user,
            &CommonServerConfig {
                auth_service_url: "https://auth.example.com".to_string(),
                secret: "secret".to_string(),
            },
        )
        .expect("shell data");

        assert!(!response.navigation.iter().any(|item| item.url == "/"));
        assert!(
            response
                .navigation
                .iter()
                .any(|item| item.url == "/managers")
        );
        assert!(
            response
                .local_menu_items
                .iter()
                .any(|item| item.url == "/settings")
        );
    }

    #[test]
    fn list_clients_requires_access_role() {
        let mut repo = MockRepository::new();
        repo.expect_list_clients().times(0);
        let mut user = access_user();
        user.roles.clear();

        let result = list_clients(ClientsQuery::default(), &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn list_clients_allows_admin_only_users() {
        let mut repo = MockRepository::new();
        let expected_client = sample_client(1, 7);
        repo.expect_list_clients()
            .withf(|query| {
                query.hub_id == HubId::new(7).expect("valid hub id")
                    && query.manager_email.is_none()
            })
            .times(1)
            .returning(move |_| Ok((1, vec![expected_client.clone()])));
        let mut user = access_user();
        user.roles = vec![SERVICE_ADMIN_ROLE.to_string()];

        let response = list_clients(ClientsQuery::default(), &user, &repo).expect("response ok");

        assert_eq!(response.total, 1);
        assert_eq!(response.clients.len(), 1);
    }

    #[test]
    fn list_clients_applies_search_and_pagination() {
        let mut repo = MockRepository::new();
        let expected_client = sample_client(1, 7);
        let response_client = expected_client.clone();
        repo.expect_list_clients()
            .withf(|query| {
                query.hub_id == HubId::new(7).expect("valid hub id")
                    && query.manager_email.is_none()
                    && query.search.as_deref() == Some("Alice")
                    && query.pagination.as_ref().is_some_and(|pagination| {
                        pagination.page == 2 && pagination.per_page == DEFAULT_ITEMS_PER_PAGE
                    })
            })
            .times(1)
            .returning(move |_| Ok((1, vec![response_client.clone()])));

        let user = access_user();
        let params = ClientsQuery {
            search: Some("  Alice  ".to_string()),
            page: Some(2),
            public_id: None,
        };

        let response = list_clients(params, &user, &repo).expect("response ok");

        assert_eq!(response.total, 1);
        assert_eq!(response.clients, vec![expected_client]);
        assert_eq!(
            response.clients[0].id,
            ClientId::new(1).expect("valid client id")
        );
        assert_eq!(
            response.clients[0].name,
            ClientName::new("Client").expect("valid name")
        );
    }

    #[test]
    fn list_clients_with_invalid_public_id_returns_empty_without_repo_query() {
        let mut repo = MockRepository::new();
        repo.expect_list_clients().times(0);

        let user = access_user();
        let params = ClientsQuery {
            public_id: Some("not-a-uuid".to_string()),
            ..Default::default()
        };

        let response = list_clients(params, &user, &repo).expect("response ok");

        assert_eq!(response.total, 0);
        assert!(response.clients.is_empty());
    }
}
