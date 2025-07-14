pub mod client;
pub mod errors;
pub mod manager;
pub mod test;

use crate::{
    domain::{
        client::{Client, NewClient, UpdateClient},
        manager::{Manager, NewManager},
    },
    pagination::Paginated,
    repository::errors::RepositoryResult,
};

pub trait ClientRepository {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Client>>;
    fn create(&self, new_clients: &[NewClient]) -> RepositoryResult<usize>;
    fn list(&self, hub_id: i32, current_page: usize) -> RepositoryResult<Paginated<Client>>;
    fn list_by_manager(
        &self,
        manager_email: &str,
        hub_id: i32,
        current_page: usize,
    ) -> RepositoryResult<Paginated<Client>>;
    fn search_paginated(
        &self,
        hub_id: i32,
        search_key: &str,
        current_page: usize,
    ) -> RepositoryResult<Paginated<Client>>;
    fn search(&self, hub_id: i32, search_key: &str) -> RepositoryResult<Vec<Client>>;
    fn update(&self, client_id: i32, updates: &UpdateClient) -> RepositoryResult<Client>;
    fn delete(&self, client_id: i32) -> RepositoryResult<()>;
}

pub trait ManagerRepository {
    fn get_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<Manager>>;
    fn create_or_update(&self, new_manager: &NewManager) -> RepositoryResult<Manager>;
    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<(Manager, Vec<Client>)>>;
    fn assign_clients(&self, manager_id: i32, client_ids: &[i32]) -> RepositoryResult<usize>;
}
