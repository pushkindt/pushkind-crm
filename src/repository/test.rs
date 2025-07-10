use crate::domain::client::{Client, NewClient, UpdateClient};
use crate::domain::manager::{Manager, NewManager};
use crate::pagination::Paginated;
use crate::repository::errors::RepositoryResult;
use crate::repository::{ClientRepository, ManagerRepository};

pub struct TestClientRepository;
pub struct TestManagerRepository;

impl ClientRepository for TestClientRepository {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Client>> {
        let client = Client {
            id,
            hub_id: 1,
            name: format!("Client Name #{id}"),
            email: format!("client#{id}@email.com"),
            phone: format!("123456789{id}"),
            address: format!("Client Address #{id}"),
            ..Client::default()
        };
        Ok(Some(client))
    }

    fn create(&self, new_clients: &[NewClient]) -> RepositoryResult<usize> {
        Ok(new_clients.len())
    }

    fn list(&self, hub_id: i32, current_page: usize) -> RepositoryResult<Paginated<Client>> {
        let clients = (1..20)
            .map(|id| Client {
                id,
                hub_id,
                name: format!("Client Name #{id}"),
                email: format!("client#{id}@email.com"),
                phone: format!("123456789{id}"),
                address: format!("Client Address #{id}"),
                ..Client::default()
            })
            .collect();
        Ok(Paginated::new(clients, current_page, 20))
    }

    fn list_by_manager(
        &self,
        _manager_email: &str,
        _hub_id: i32,
        current_page: usize,
    ) -> RepositoryResult<Paginated<Client>> {
        let clients = (1..20)
            .map(|id| Client {
                id,
                hub_id: 1,
                name: format!("Client Name #{id}"),
                email: format!("client#{id}@email.com"),
                phone: format!("123456789{id}"),
                address: format!("Client Address #{id}"),
                ..Client::default()
            })
            .collect();
        Ok(Paginated::new(clients, current_page, 20))
    }

    fn search(
        &self,
        hub_id: i32,
        search_key: &str,
        current_page: usize,
    ) -> RepositoryResult<Paginated<Client>> {
        let clients = (1..20)
            .map(|id| Client {
                id,
                hub_id,
                name: format!("Client Name #{id}"),
                email: format!("client#{id}@email.com"),
                phone: format!("123456789{id}"),
                address: format!("Client Address #{search_key}"),
                ..Client::default()
            })
            .collect();
        Ok(Paginated::new(clients, current_page, 3))
    }

    fn update(&self, client_id: i32, updates: &UpdateClient) -> RepositoryResult<Client> {
        let client = Client {
            id: client_id,
            name: updates.name.to_string(),
            email: updates.email.to_string(),
            phone: updates.phone.to_string(),
            address: updates.address.to_string(),
            ..Client::default()
        };

        Ok(client)
    }

    fn delete(&self, _client_id: i32) -> RepositoryResult<()> {
        Ok(())
    }
}

impl ManagerRepository for TestManagerRepository {
    fn get_by_email(
        &self,
        _email: &str,
        _hub_id: i32,
    ) -> RepositoryResult<Option<crate::domain::manager::Manager>> {
        Ok(None)
    }
    fn create_or_update(&self, new_manager: &NewManager) -> RepositoryResult<Manager> {
        let manager = Manager {
            id: 1,
            hub_id: new_manager.hub_id,
            name: new_manager.name.to_string(),
            email: new_manager.email.to_string(),
        };
        Ok(manager)
    }

    fn list(&self, _hub_id: i32) -> RepositoryResult<Vec<(Manager, Vec<Client>)>> {
        Ok(vec![])
    }
}
