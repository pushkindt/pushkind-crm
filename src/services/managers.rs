//! Services handling manager administration workflows.

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::check_role;
use validator::Validate;

use crate::SERVICE_ADMIN_ROLE;
use crate::domain::manager::NewManager;
use crate::dto::managers::{ManagerModalData, ManagersPageData};
use crate::forms::managers::{AddManagerForm, AssignManagerForm};
use crate::repository::{ClientListQuery, ClientReader, ManagerReader, ManagerWriter};
use crate::services::client as client_service;
use crate::services::{ServiceError, ServiceResult};

/// Loads all managers with the clients assigned to them.
pub fn list_managers<R>(repo: &R, user: &AuthenticatedUser) -> ServiceResult<ManagersPageData>
where
    R: ManagerReader + ?Sized,
{
    if !check_role(SERVICE_ADMIN_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let managers =
        client_service::list_managers_with_clients(repo, user.hub_id).map_err(|err| {
            log::error!("Failed to list managers: {err}");
            err
        })?;

    Ok(ManagersPageData { managers })
}

/// Validates the incoming form and persists the manager entity.
pub fn add_manager<R>(repo: &R, user: &AuthenticatedUser, form: AddManagerForm) -> ServiceResult<()>
where
    R: ManagerWriter + ?Sized,
{
    if !check_role(SERVICE_ADMIN_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    if let Err(err) = form.validate() {
        log::error!("Failed to validate form: {err}");
        return Err(ServiceError::Form("Ошибка валидации формы".to_string()));
    }

    let new_manager = NewManager::try_from_parts(user.hub_id, form.name, form.email, true)
        .map_err(|err| {
            log::error!("Invalid manager payload: {err}");
            ServiceError::Form("Ошибка валидации формы".to_string())
        })?;

    client_service::create_or_update_manager(repo, &new_manager).map_err(|err| {
        log::error!("Failed to save the manager: {err}");
        err
    })?;

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
    if !check_role(SERVICE_ADMIN_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let manager = repo
        .get_manager_by_id(manager_id, user.hub_id)
        .map_err(ServiceError::from)?
        .ok_or_else(|| {
            log::error!(
                "Manager {manager_id} not found for hub {hub_id}",
                hub_id = user.hub_id
            );
            ServiceError::NotFound
        })?;

    let (_, clients) = repo
        .list_clients(ClientListQuery::new(user.hub_id).manager_email(manager.email.as_str()))
        .map_err(ServiceError::from)?;

    Ok(ManagerModalData { manager, clients })
}

/// Assigns the provided client identifiers to the given manager.
pub fn assign_manager<R>(repo: &R, user: &AuthenticatedUser, payload: &[u8]) -> ServiceResult<()>
where
    R: ManagerReader + ManagerWriter + ?Sized,
{
    if !check_role(SERVICE_ADMIN_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let form: AssignManagerForm = serde_html_form::from_bytes(payload).map_err(|err| {
        log::error!("Failed to process form: {err}");
        ServiceError::Form("Ошибка при обработке формы".to_string())
    })?;

    let manager = repo
        .get_manager_by_id(form.manager_id, user.hub_id)
        .map_err(ServiceError::from)?
        .ok_or_else(|| {
            log::error!(
                "Manager {manager_id} not found for hub {hub_id}",
                manager_id = form.manager_id,
                hub_id = user.hub_id
            );
            ServiceError::NotFound
        })?;

    client_service::assign_clients_to_manager(repo, manager.id.get(), &form.client_ids).map_err(
        |err| {
            log::error!("Failed to assign clients to the manager: {err}");
            err
        },
    )?;

    Ok(())
}
