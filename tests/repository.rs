use pushkind_crm::domain::client::{Client, NewClient, UpdateClient};
use pushkind_crm::domain::client_event::{ClientEvent, ClientEventType, NewClientEvent};
use pushkind_crm::domain::manager::{
    ClientManager, Manager, NewClientManager, NewManager, UpdateManager,
};
use pushkind_crm::repository::client::DieselClientRepository;
use pushkind_crm::repository::client_event::DieselClientEventRepository;
use pushkind_crm::repository::manager::DieselManagerRepository;
use pushkind_crm::repository::{ClientEventListQuery, ClientEventReader, ClientEventWriter};
use pushkind_crm::repository::{ClientListQuery, ClientReader, ClientSearchQuery, ClientWriter};
use pushkind_crm::repository::{ManagerReader, ManagerWriter};

mod common;

#[test]
fn test_client_repository_crud() {
    let test_db = common::TestDb::new("test_client_repository_crud.db");
    let client_repo = DieselClientRepository::new(test_db.pool());
}

#[test]
fn test_client_event_repository_crud() {
    let test_db = common::TestDb::new("test_client_event_repository_crud.db");
    let client_event_repo = DieselClientEventRepository::new(test_db.pool());
}

#[test]
fn test_manager_repository_crud() {
    let test_db = common::TestDb::new("test_manager_repository_crud.db");
    let manager_repo = DieselManagerRepository::new(test_db.pool());
}
