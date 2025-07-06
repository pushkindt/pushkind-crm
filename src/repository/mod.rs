pub mod errors;
pub mod test;

use crate::{
    domain::client::{Client, NewClient, UpdateClient},
    pagination::Paginated,
    repository::errors::RepositoryResult,
};

pub trait ClientRepository {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Client>>;
    fn create(&self, new_client: &NewClient) -> RepositoryResult<Client>;
    fn list(&self, hub_id: i32, current_page: usize) -> RepositoryResult<Paginated<Client>>;
    fn update(&self, client_id: i32, updates: &UpdateClient) -> RepositoryResult<Client>;
    fn delete(&self, client_id: i32) -> RepositoryResult<()>;
}
