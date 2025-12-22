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
pub fn list_managers<R>(repo: &R, user: &AuthenticatedUser) -> ServiceResult<ManagersPageData>
where
    R: ManagerReader + ?Sized,
{
    ensure_role(user, SERVICE_ADMIN_ROLE)?;

    let hub_id = HubId::new(user.hub_id)?;

    let managers = repo.list_managers_with_clients(hub_id)?;

    Ok(ManagersPageData { managers })
}

/// Validates the incoming form and persists the manager entity.
pub fn add_manager<R>(repo: &R, user: &AuthenticatedUser, form: AddManagerForm) -> ServiceResult<()>
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
    repo: &R,
    user: &AuthenticatedUser,
    manager_id: i32,
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
    repo: &R,
    user: &AuthenticatedUser,
    form: AssignManagerForm,
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
