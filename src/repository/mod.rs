//! Repository traits and Diesel implementation for the CRM domain.

use pushkind_common::db::{DbConnection, DbPool};
use pushkind_common::pagination::Pagination;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::types::{ClientEmail, ClientId, HubId, ManagerEmail, ManagerId, PublicId};
use crate::domain::{
    client::{Client, NewClient, UpdateClient},
    client_event::{ClientEvent, ClientEventType, NewClientEvent},
    important_field::ImportantField as DomainImportantField,
    manager::{Manager, NewManager},
};

pub mod client;
pub mod client_event;
pub mod manager;
#[cfg(feature = "test-mocks")]
pub mod mock;

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
    pub hub_id: HubId,
    pub manager_email: Option<ManagerEmail>,
    pub search: Option<String>,
    pub public_id: Option<PublicId>,
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Clone)]
pub struct ClientEventListQuery {
    pub client_id: ClientId,
    pub event_type: Option<ClientEventType>,
    pub pagination: Option<Pagination>,
}

impl ClientListQuery {
    pub fn new(hub_id: HubId) -> Self {
        Self {
            hub_id,
            manager_email: None,
            search: None,
            public_id: None,
            pagination: None,
        }
    }

    pub fn manager_email(mut self, email: ManagerEmail) -> Self {
        self.manager_email = Some(email);
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

    pub fn public_id(mut self, public_id: PublicId) -> Self {
        self.public_id = Some(public_id);
        self
    }
}

impl ClientEventListQuery {
    pub fn new(client_id: ClientId) -> Self {
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
    fn get_client_by_public_id(
        &self,
        public_id: PublicId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Client>>;
    fn get_client_by_id(&self, id: ClientId, hub_id: HubId) -> RepositoryResult<Option<Client>>;
    fn get_client_by_email(
        &self,
        email: &ClientEmail,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Client>>;
    fn list_clients(&self, query: ClientListQuery) -> RepositoryResult<(usize, Vec<Client>)>;
    fn list_managers(&self, id: ClientId) -> RepositoryResult<Vec<Manager>>;
    fn check_client_assigned_to_manager(
        &self,
        client_id: ClientId,
        manager_email: &ManagerEmail,
    ) -> RepositoryResult<bool>;
    fn list_available_fields(&self, hub_id: HubId) -> RepositoryResult<Vec<String>>;
}

pub trait ClientWriter {
    fn create_or_replace_clients(&self, new_clients: &[NewClient]) -> RepositoryResult<usize>;
    fn create_clients(&self, new_clients: &[NewClient]) -> RepositoryResult<usize>;
    fn update_client(
        &self,
        client_id: ClientId,
        updates: &UpdateClient,
    ) -> RepositoryResult<Client>;
    fn delete_client(&self, client_id: ClientId) -> RepositoryResult<()>;
    fn delete_all_clients(&self, hub_id: HubId) -> RepositoryResult<()>;
}

pub trait ImportantFieldReader {
    fn list_important_fields(&self, hub_id: HubId) -> RepositoryResult<Vec<DomainImportantField>>;
}

pub trait ImportantFieldWriter {
    fn replace_important_fields(
        &self,
        hub_id: HubId,
        fields: &[DomainImportantField],
    ) -> RepositoryResult<()>;
}
pub trait ManagerReader {
    fn get_manager_by_id(&self, id: ManagerId, hub_id: HubId) -> RepositoryResult<Option<Manager>>;
    fn get_manager_by_email(
        &self,
        email: &ManagerEmail,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Manager>>;
    fn list_managers_with_clients(
        &self,
        hub_id: HubId,
    ) -> RepositoryResult<Vec<(Manager, Vec<Client>)>>;
}

pub trait ManagerWriter {
    fn create_or_update_manager(&self, new_manager: &NewManager) -> RepositoryResult<Manager>;
    fn assign_clients_to_manager(
        &self,
        manager_id: ManagerId,
        client_ids: &[ClientId],
    ) -> RepositoryResult<usize>;
}

pub trait ClientEventReader {
    fn list_client_events(
        &self,
        query: ClientEventListQuery,
    ) -> RepositoryResult<(usize, Vec<(ClientEvent, Manager)>)>;
    fn client_event_exists(&self, event: &NewClientEvent) -> RepositoryResult<bool>;
}

pub trait ClientEventWriter {
    fn create_client_event(&self, client_event: &NewClientEvent) -> RepositoryResult<ClientEvent>;
}
