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
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Client>>;
    fn list(&self, query: ClientListQuery) -> RepositoryResult<(usize, Vec<Client>)>;
    fn search(&self, query: ClientListQuery) -> RepositoryResult<(usize, Vec<Client>)>;
    fn list_managers(&self, id: i32) -> RepositoryResult<Vec<Manager>>;
    fn check_manager_assigned(&self, client_id: i32, manager_email: &str)
    -> RepositoryResult<bool>;
}

pub trait ClientWriter {
    fn create(&self, new_clients: &[NewClient]) -> RepositoryResult<usize>;
    fn update(&self, client_id: i32, updates: &UpdateClient) -> RepositoryResult<Client>;
    fn delete(&self, client_id: i32) -> RepositoryResult<()>;
}
pub trait ManagerReader {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Manager>>;
    fn get_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<Manager>>;
    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<(Manager, Vec<Client>)>>;
}

pub trait ManagerWriter {
    fn create_or_update(&self, new_manager: &NewManager) -> RepositoryResult<Manager>;
    fn assign_clients(&self, manager_id: i32, client_ids: &[i32]) -> RepositoryResult<usize>;
}

pub trait ClientEventReader {
    fn list(
        &self,
        query: ClientEventListQuery,
    ) -> RepositoryResult<(usize, Vec<(ClientEvent, Manager)>)>;
}

pub trait ClientEventWriter {
    fn create(&self, client_event: &NewClientEvent) -> RepositoryResult<ClientEvent>;
}
