use crate::domain::client::{Client, NewClient, UpdateClient};
use crate::domain::client_event::{ClientEvent, NewClientEvent};
use crate::domain::manager::{Manager, NewManager};
use crate::repository::{
    ClientEventListQuery, ClientEventReader, ClientEventWriter, ClientReader, ClientWriter,
    ManagerReader, ManagerWriter,
};
use crate::services::{ServiceError, ServiceResult};

/// Fetches a client by its identifier scoped to the provided hub.
pub fn get_client_by_id<R>(repo: &R, client_id: i32, hub_id: i32) -> ServiceResult<Option<Client>>
where
    R: ClientReader + ?Sized,
{
    repo.get_client_by_id(client_id, hub_id)
        .map_err(ServiceError::from)
}

/// Returns the managers linked to the given client.
pub fn list_client_managers<R>(repo: &R, client_id: i32) -> ServiceResult<Vec<Manager>>
where
    R: ClientReader + ?Sized,
{
    repo.list_managers(client_id).map_err(ServiceError::from)
}

/// Retrieves the paginated list of client events with their managers.
pub fn list_client_events<R>(
    repo: &R,
    query: ClientEventListQuery,
) -> ServiceResult<(usize, Vec<(ClientEvent, Manager)>)>
where
    R: ClientEventReader + ?Sized,
{
    repo.list_client_events(query).map_err(ServiceError::from)
}

/// Checks whether the client is assigned to the specified manager email.
pub fn is_client_assigned_to_manager<R>(
    repo: &R,
    client_id: i32,
    manager_email: &str,
) -> ServiceResult<bool>
where
    R: ClientReader + ?Sized,
{
    repo.check_client_assigned_to_manager(client_id, manager_email)
        .map_err(ServiceError::from)
}

/// Applies the provided updates to the client entity.
pub fn update_client<R>(repo: &R, client_id: i32, updates: &UpdateClient) -> ServiceResult<Client>
where
    R: ClientWriter + ?Sized,
{
    repo.update_client(client_id, updates)
        .map_err(ServiceError::from)
}

/// Persists or updates the manager derived from the provided data.
pub fn create_or_update_manager<R>(repo: &R, new_manager: &NewManager) -> ServiceResult<Manager>
where
    R: ManagerWriter + ?Sized,
{
    repo.create_or_update_manager(new_manager)
        .map_err(ServiceError::from)
}

/// Persists a new client event.
pub fn create_client_event<R>(repo: &R, event: &NewClientEvent) -> ServiceResult<ClientEvent>
where
    R: ClientEventWriter + ?Sized,
{
    repo.create_client_event(event).map_err(ServiceError::from)
}

/// Lists all managers for the provided hub with their assigned clients.
pub fn list_managers_with_clients<R>(
    repo: &R,
    hub_id: i32,
) -> ServiceResult<Vec<(Manager, Vec<Client>)>>
where
    R: ManagerReader + ?Sized,
{
    repo.list_managers_with_clients(hub_id)
        .map_err(ServiceError::from)
}

/// Creates a batch of clients returning the count of inserted rows.
pub fn create_clients<R>(repo: &R, new_clients: &[NewClient]) -> ServiceResult<usize>
where
    R: ClientWriter + ?Sized,
{
    repo.create_clients(new_clients).map_err(ServiceError::from)
}

/// Assigns the provided list of client identifiers to the given manager.
pub fn assign_clients_to_manager<R>(
    repo: &R,
    manager_id: i32,
    client_ids: &[i32],
) -> ServiceResult<usize>
where
    R: ManagerWriter + ?Sized,
{
    repo.assign_clients_to_manager(manager_id, client_ids)
        .map_err(ServiceError::from)
}
