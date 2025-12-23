//! Mock repository implementations for isolating services in tests.

use mockall::mock;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::client::{Client, NewClient, UpdateClient};
use crate::domain::client_event::{ClientEvent, NewClientEvent};
use crate::domain::important_field::ImportantField;
use crate::domain::manager::{Manager, NewManager};
use crate::domain::types::{ClientEmail, ClientId, HubId, ManagerEmail, ManagerId};
use crate::repository::{
    ClientEventListQuery, ClientEventReader, ClientEventWriter, ClientListQuery, ClientReader,
    ClientWriter, ImportantFieldReader, ImportantFieldWriter, ManagerReader, ManagerWriter,
};

mock! {
    pub Repository {}

    impl ClientReader for Repository {
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

    impl ManagerReader for Repository {
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

    impl ImportantFieldReader for Repository {
        fn list_important_fields(&self, hub_id: HubId) -> RepositoryResult<Vec<ImportantField>>;
    }

    impl ClientEventReader for Repository {
        fn list_client_events(
            &self,
            query: ClientEventListQuery,
        ) -> RepositoryResult<(usize, Vec<(ClientEvent, Manager)>)>;
        fn client_event_exists(&self, event: &NewClientEvent) -> RepositoryResult<bool>;
    }

    impl ClientWriter for Repository {
        fn create_clients(&self, new_clients: &[NewClient]) -> RepositoryResult<usize>;
        fn update_client(
            &self,
            client_id: ClientId,
            updates: &UpdateClient,
        ) -> RepositoryResult<Client>;
        fn delete_client(&self, client_id: ClientId) -> RepositoryResult<()>;
    }

    impl ManagerWriter for Repository {
        fn create_or_update_manager(&self, new_manager: &NewManager) -> RepositoryResult<Manager>;
        fn assign_clients_to_manager(
            &self,
            manager_id: ManagerId,
            client_ids: &[ClientId],
        ) -> RepositoryResult<usize>;
    }

    impl ImportantFieldWriter for Repository {
        fn replace_important_fields(
            &self,
            hub_id: HubId,
            fields: &[ImportantField],
        ) -> RepositoryResult<()>;
    }

    impl ClientEventWriter for Repository {
        fn create_client_event(&self, client_event: &NewClientEvent) -> RepositoryResult<ClientEvent>;
    }
}
