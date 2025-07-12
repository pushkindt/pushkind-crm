use diesel::prelude::*;

use crate::{
    db::DbPool,
    domain::client::{Client, NewClient, UpdateClient},
    pagination::Paginated,
    repository::{ClientRepository, errors::RepositoryResult},
};

/// Diesel implementation of [`ClientRepository`].
pub struct DieselClientRepository<'a> {
    pub pool: &'a DbPool,
}

impl<'a> DieselClientRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl ClientRepository for DieselClientRepository<'_> {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Client>> {}
    fn create(&self, new_clients: &[NewClient]) -> RepositoryResult<usize> {}
    fn list(&self, hub_id: i32, current_page: usize) -> RepositoryResult<Paginated<Client>> {}
    fn list_by_manager(
        &self,
        manager_email: &str,
        hub_id: i32,
        current_page: usize,
    ) -> RepositoryResult<Paginated<Client>> {
    }
    fn search(
        &self,
        hub_id: i32,
        search_key: &str,
        current_page: usize,
    ) -> RepositoryResult<Paginated<Client>> {
    }
    fn update(&self, client_id: i32, updates: &UpdateClient) -> RepositoryResult<Client> {}
    fn delete(&self, client_id: i32) -> RepositoryResult<()> {}
}
