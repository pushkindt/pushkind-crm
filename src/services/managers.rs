//! Services handling manager administration workflows.

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::ensure_role;

use crate::SERVICE_ADMIN_ROLE;
use crate::domain::types::{HubId, ManagerId};
use crate::dto::managers::{ManagerModalData, ManagersPageData};
use crate::forms::managers::{
    AddManagerForm, AddManagerPayload, AssignManagerForm, AssignManagerPayload,
};
use crate::repository::{ClientListQuery, ClientReader, ManagerReader, ManagerWriter};
use crate::services::{ServiceError, ServiceResult};

/// Loads all managers with the clients assigned to them.
pub fn list_managers<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<ManagersPageData>
where
    R: ManagerReader + ?Sized,
{
    ensure_role(user, SERVICE_ADMIN_ROLE)?;

    let hub_id = HubId::new(user.hub_id)?;

    let managers = repo.list_managers_with_clients(hub_id)?;

    Ok(ManagersPageData { managers })
}

/// Validates the incoming form and persists the manager entity.
pub fn add_manager<R>(form: AddManagerForm, user: &AuthenticatedUser, repo: &R) -> ServiceResult<()>
where
    R: ManagerWriter + ?Sized,
{
    ensure_role(user, SERVICE_ADMIN_ROLE)?;

    let payload = AddManagerPayload::try_from(form)?;

    let hub_id = HubId::new(user.hub_id)?;

    let new_manager = payload.into_domain(hub_id);

    repo.create_or_update_manager(&new_manager)?;

    Ok(())
}

/// Loads data necessary to render the manager modal body.
pub fn load_manager_modal<R>(
    manager_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ManagerModalData>
where
    R: ManagerReader + ClientReader + ?Sized,
{
    ensure_role(user, SERVICE_ADMIN_ROLE)?;

    let hub_id = HubId::new(user.hub_id)?;

    let manager = repo
        .get_manager_by_id(ManagerId::new(manager_id)?, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let (_, clients) = repo
        .list_clients(ClientListQuery::new(hub_id).manager_email(manager.email.clone()))
        .map_err(ServiceError::from)?;

    Ok(ManagerModalData { manager, clients })
}

/// Assigns the provided client identifiers to the given manager.
pub fn assign_manager<R>(
    form: AssignManagerForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: ClientReader + ManagerReader + ManagerWriter + ?Sized,
{
    ensure_role(user, SERVICE_ADMIN_ROLE)?;

    let payload = AssignManagerPayload::try_from(form)?;

    let hub_id = HubId::new(user.hub_id)?;

    let manager = repo
        .get_manager_by_id(payload.manager_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    for client_id in &payload.client_ids {
        if repo.get_client_by_id(*client_id, hub_id)?.is_none() {
            return Err(ServiceError::Form(
                "Некорректный список клиентов".to_string(),
            ));
        }
    }

    repo.assign_clients_to_manager(manager.id, &payload.client_ids)?;

    Ok(())
}

#[cfg(all(test, feature = "test-mocks"))]
mod tests {
    use super::*;
    use crate::domain::client::Client;
    use crate::domain::manager::Manager;
    use crate::domain::types::{ClientId, HubId, ManagerEmail, ManagerId, ManagerName};
    use crate::repository::mock::MockRepository;
    use chrono::Utc;
    use pushkind_common::services::errors::ServiceError;

    fn admin_user() -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "1".to_string(),
            email: "admin@example.com".to_string(),
            hub_id: 22,
            name: "Admin".to_string(),
            roles: vec![SERVICE_ADMIN_ROLE.to_string()],
            exp: 0,
        }
    }

    fn viewer_user() -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "2".to_string(),
            email: "viewer@example.com".to_string(),
            hub_id: 22,
            name: "Viewer".to_string(),
            roles: vec!["crm".to_string()],
            exp: 0,
        }
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
    fn list_managers_requires_admin_role() {
        let mut repo = MockRepository::new();
        repo.expect_list_managers_with_clients().times(0);
        let user = viewer_user();

        let result = list_managers(&user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn add_manager_creates_manager() {
        let mut repo = MockRepository::new();
        let manager = sample_manager(1, 22);
        repo.expect_create_or_update_manager()
            .withf(|payload| {
                payload.hub_id == HubId::new(22).expect("valid hub id")
                    && payload.is_user
                    && payload.email == ManagerEmail::new("manager@example.com").expect("email")
                    && payload.name == ManagerName::new("Manager").expect("name")
            })
            .times(1)
            .returning(move |_| Ok(manager.clone()));
        let user = admin_user();
        let form = AddManagerForm {
            name: "Manager".to_string(),
            email: "manager@example.com".to_string(),
        };

        add_manager(form, &user, &repo).expect("manager created");
    }

    #[test]
    fn load_manager_modal_returns_data() {
        let mut repo = MockRepository::new();
        let manager = sample_manager(5, 22);
        let client = sample_client(7, 22);
        let expected_client = client.clone();
        repo.expect_get_manager_by_id()
            .withf(|manager_id, hub_id| {
                manager_id == &ManagerId::new(5).expect("manager id")
                    && hub_id == &HubId::new(22).expect("hub id")
            })
            .times(1)
            .returning(move |_, _| Ok(Some(manager.clone())));
        repo.expect_list_clients()
            .withf(|query| {
                query.hub_id == HubId::new(22).expect("valid hub id")
                    && query
                        .manager_email
                        .as_ref()
                        .map(|email| email.as_str())
                        == Some("manager@example.com")
            })
            .times(1)
            .returning(move |_| Ok((1, vec![expected_client.clone()])));

        let user = admin_user();
        let data = load_manager_modal(5, &user, &repo).expect("modal data");

        assert_eq!(data.manager.email.as_str(), "manager@example.com");
        assert_eq!(data.clients, vec![client]);
    }

    #[test]
    fn assign_manager_rejects_unknown_clients() {
        let mut repo = MockRepository::new();
        let manager = sample_manager(1, 22);
        repo.expect_get_manager_by_id()
            .times(1)
            .returning(move |_, _| Ok(Some(manager.clone())));
        let valid_client_id = ClientId::new(1).expect("client id");
        repo.expect_get_client_by_id()
            .times(2)
            .returning(move |client_id, _| {
                if client_id == valid_client_id {
                    Ok(Some(sample_client(1, 22)))
                } else {
                    Ok(None)
                }
            });
        repo.expect_assign_clients_to_manager().times(0);
        let user = admin_user();
        let form = AssignManagerForm {
            manager_id: 1,
            client_ids: vec![1, 2],
        };

        let result = assign_manager(form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn assign_manager_assigns_clients() {
        let mut repo = MockRepository::new();
        let manager = sample_manager(2, 22);
        repo.expect_get_manager_by_id()
            .times(1)
            .returning(move |_, _| Ok(Some(manager.clone())));
        repo.expect_get_client_by_id()
            .times(2)
            .returning(move |client_id, _| Ok(Some(sample_client(client_id.get(), 22))));
        repo.expect_assign_clients_to_manager()
            .withf(|manager_id, client_ids| {
                manager_id == &ManagerId::new(2).expect("manager id")
                    && client_ids.len() == 2
            })
            .times(1)
            .returning(|_, _| Ok(2));
        let user = admin_user();
        let form = AssignManagerForm {
            manager_id: 2,
            client_ids: vec![3, 4],
        };

        assign_manager(form, &user, &repo).expect("assignment ok");
    }
}
