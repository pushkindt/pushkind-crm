use pushkind_common::db::{DbConnection, DbPool};
use pushkind_common::pagination::Pagination;

use crate::{
    domain::{
        client::{Client, NewClient, UpdateClient},
        client_event::{ClientEvent, ClientEventType, NewClientEvent},
        manager::{Manager, NewManager},
    },
    repository::errors::RepositoryResult,
};

pub mod client;
pub mod client_event;
pub mod errors;
pub mod manager;

#[derive(Clone)]
pub struct DieselRepository {
    pool: DbPool, // r2d2::Pool is cheap to clone
}

impl DieselRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn conn(&self) -> RepositoryResult<DbConnection> {
        Ok(self.pool.get()?)
    }
}

#[derive(Debug, Clone)]
pub struct ClientListQuery {
    pub hub_id: i32,
    pub manager_email: Option<String>,
    pub search: Option<String>,
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Clone)]
pub struct ClientEventListQuery {
    pub client_id: i32,
    pub event_type: Option<ClientEventType>,
    pub pagination: Option<Pagination>,
}

impl ClientListQuery {
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            manager_email: None,
            search: None,
            pagination: None,
        }
    }

    pub fn manager_email(mut self, email: impl Into<String>) -> Self {
        self.manager_email = Some(email.into());
        self
    }

    pub fn search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }

    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}

impl ClientEventListQuery {
    pub fn new(client_id: i32) -> Self {
        Self {
            client_id,
            event_type: None,
            pagination: None,
        }
    }

    pub fn event_type(mut self, event_type: ClientEventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}

pub trait ClientReader {
    fn get_client_by_id(&self, id: i32) -> RepositoryResult<Option<Client>>;
    fn list_clients(&self, query: ClientListQuery) -> RepositoryResult<(usize, Vec<Client>)>;
    fn search_clients(&self, query: ClientListQuery) -> RepositoryResult<(usize, Vec<Client>)>;
    fn list_managers(&self, id: i32) -> RepositoryResult<Vec<Manager>>;
    fn check_client_assigned_to_manager(
        &self,
        client_id: i32,
        manager_email: &str,
    ) -> RepositoryResult<bool>;
}

pub trait ClientWriter {
    fn create_clients(&self, new_clients: &[NewClient]) -> RepositoryResult<usize>;
    fn update_client(&self, client_id: i32, updates: &UpdateClient) -> RepositoryResult<Client>;
    fn delete_client(&self, client_id: i32) -> RepositoryResult<()>;
}
pub trait ManagerReader {
    fn get_manager_by_id(&self, id: i32) -> RepositoryResult<Option<Manager>>;
    fn get_manager_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<Manager>>;
    fn list_managers_with_clients(
        &self,
        hub_id: i32,
    ) -> RepositoryResult<Vec<(Manager, Vec<Client>)>>;
}

pub trait ManagerWriter {
    fn create_or_update_manager(&self, new_manager: &NewManager) -> RepositoryResult<Manager>;
    fn assign_clients_to_manager(
        &self,
        manager_id: i32,
        client_ids: &[i32],
    ) -> RepositoryResult<usize>;
}

pub trait ClientEventReader {
    fn list_client_events(
        &self,
        query: ClientEventListQuery,
    ) -> RepositoryResult<(usize, Vec<(ClientEvent, Manager)>)>;
}

pub trait ClientEventWriter {
    fn create_client_event(&self, client_event: &NewClientEvent) -> RepositoryResult<ClientEvent>;
}
