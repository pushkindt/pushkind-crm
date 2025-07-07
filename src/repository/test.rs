use crate::domain::client::{Client, NewClient, UpdateClient};
use crate::pagination::Paginated;
use crate::repository::ClientRepository;
use crate::repository::errors::RepositoryResult;

pub struct TestClientRepository;

impl ClientRepository for TestClientRepository {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Client>> {
        let mut client = Client::default();
        client.id = id;
        Ok(Some(client))
    }

    fn create(&self, new_client: &NewClient) -> RepositoryResult<Client> {
        let client = Client {
            hub_id: new_client.hub_id,
            name: new_client.name.clone(),
            email: new_client.email.clone(),
            phone: new_client.phone.clone(),
            address: new_client.address.clone(),
            ..Client::default()
        };

        Ok(client)
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
            name: updates.name.clone(),
            email: updates.email.clone(),
            phone: updates.phone.clone(),
            address: updates.address.clone(),
            ..Client::default()
        };

        Ok(client)
    }

    fn delete(&self, _client_id: i32) -> RepositoryResult<()> {
        Ok(())
    }
}
