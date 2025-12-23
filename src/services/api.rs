//! Service adaptors serving CRM API data.

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::routes::ensure_role;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::types::HubId;
pub use crate::dto::api::{ClientsQuery, ClientsResponse};
use crate::repository::{ClientListQuery, ClientReader};
use crate::services::{ServiceError, ServiceResult};

/// Returns the filtered list of clients visible to the authenticated user.
pub fn list_clients<R>(
    params: ClientsQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ClientsResponse>
where
    R: ClientReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

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

    let (total, clients) = repo.list_clients(query).map_err(ServiceError::from)?;

    Ok(ClientsResponse { total, clients })
}

#[cfg(all(test, feature = "test-mocks"))]
mod tests {
    use super::*;
    use crate::domain::client::Client;
    use crate::domain::types::{ClientId, ClientName, HubId};
    use crate::repository::mock::MockRepository;
    use chrono::Utc;
    use pushkind_common::services::errors::ServiceError;

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
    fn list_clients_requires_access_role() {
        let mut repo = MockRepository::new();
        repo.expect_list_clients().times(0);
        let mut user = access_user();
        user.roles.clear();

        let result = list_clients(ClientsQuery::default(), &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
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
                    && query
                        .pagination
                        .as_ref()
                        .is_some_and(|pagination| {
                            pagination.page == 2
                                && pagination.per_page == DEFAULT_ITEMS_PER_PAGE
                        })
            })
            .times(1)
            .returning(move |_| Ok((1, vec![response_client.clone()])));

        let user = access_user();
        let params = ClientsQuery {
            search: Some("  Alice  ".to_string()),
            page: Some(2),
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
}
