use chrono::Utc;
use pushkind_crm::domain::client::{NewClient, UpdateClient};
use pushkind_crm::domain::client_event::{ClientEventType, NewClientEvent};
use pushkind_crm::domain::manager::NewManager;
use pushkind_crm::repository::client::DieselClientRepository;
use pushkind_crm::repository::client_event::DieselClientEventRepository;
use pushkind_crm::repository::manager::DieselManagerRepository;
use pushkind_crm::repository::{ClientEventListQuery, ClientEventReader, ClientEventWriter};
use pushkind_crm::repository::{ClientListQuery, ClientReader, ClientWriter};
use pushkind_crm::repository::{ManagerReader, ManagerWriter};
use serde_json::json;

mod common;

#[test]
fn test_client_repository_crud() {
    let test_db = common::TestDb::new("test_client_repository_crud.db");
    let client_repo = DieselClientRepository::new(test_db.pool());
    let c1 = NewClient {
        hub_id: 1,
        name: "Alice".into(),
        email: "alice@example.com".into(),
        phone: "111".into(),
        address: "Addr1".into(),
    };
    let c2 = NewClient {
        hub_id: 1,
        name: "Bob".into(),
        email: "bob@example.com".into(),
        phone: "222".into(),
        address: "Addr2".into(),
    };

    assert_eq!(client_repo.create(&[c1.clone(), c2.clone()]).unwrap(), 2);

    let (total, mut items) = client_repo.list(ClientListQuery::new(1)).unwrap();
    assert_eq!(total, 2);
    assert_eq!(items.len(), 2);
    items.sort_by(|a, b| a.name.cmp(&b.name));
    let alice = items[0].clone();
    let bob = items[1].clone();

    let (search_total, search_items) = client_repo
        .search(ClientListQuery::new(1).search("Bob"))
        .unwrap();
    assert_eq!(search_total, 1);
    assert_eq!(search_items[0].name, "Bob");

    let updates = UpdateClient {
        name: "Bobby",
        email: &bob.email,
        phone: &bob.phone,
        address: &bob.address,
    };
    let updated = client_repo.update(bob.id, &updates).unwrap();
    assert_eq!(updated.name, "Bobby");

    client_repo.delete(alice.id).unwrap();
    assert!(client_repo.get_by_id(alice.id).unwrap().is_none());

    let (total_after, items_after) = client_repo.list(ClientListQuery::new(1)).unwrap();
    assert_eq!(total_after, 1);
    assert_eq!(items_after[0].name, "Bobby");
}

#[test]
fn test_client_event_repository_crud() {
    let test_db = common::TestDb::new("test_client_event_repository_crud.db");
    let client_repo = DieselClientRepository::new(test_db.pool());
    let manager_repo = DieselManagerRepository::new(test_db.pool());
    let client = {
        let new_client = NewClient {
            hub_id: 1,
            name: "Alice".into(),
            email: "alice@example.com".into(),
            phone: "111".into(),
            address: "Addr1".into(),
        };
        client_repo.create(&[new_client]).unwrap();
        client_repo
            .list(ClientListQuery::new(1))
            .unwrap()
            .1
            .remove(0)
    };
    let manager = manager_repo
        .create_or_update(&NewManager {
            hub_id: 1,
            name: "Manager",
            email: "m@example.com",
        })
        .unwrap();

    let client_event_repo = DieselClientEventRepository::new(test_db.pool());

    let new_event = NewClientEvent {
        client_id: client.id,
        manager_id: manager.id,
        event_type: ClientEventType::Comment,
        event_data: json!({"text": "hello"}),
        created_at: Utc::now().naive_utc(),
    };
    let created = client_event_repo.create(&new_event).unwrap();
    assert_eq!(created.event_type, ClientEventType::Comment);

    let _ = client_event_repo
        .create(&NewClientEvent {
            client_id: client.id,
            manager_id: manager.id,
            event_type: ClientEventType::Call,
            event_data: json!({}),
            created_at: Utc::now().naive_utc(),
        })
        .unwrap();

    let (total, events) = client_event_repo
        .list(ClientEventListQuery::new(client.id))
        .unwrap();
    assert_eq!(total, 2);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].1.id, manager.id);

    let (total_comment, comments) = client_event_repo
        .list(ClientEventListQuery::new(client.id).event_type(ClientEventType::Comment))
        .unwrap();
    assert_eq!(total_comment, 1);
    assert_eq!(comments[0].0.event_type, ClientEventType::Comment);
}

#[test]
fn test_manager_repository_crud() {
    let test_db = common::TestDb::new("test_manager_repository_crud.db");
    let client_repo = DieselClientRepository::new(test_db.pool());
    let manager_repo = DieselManagerRepository::new(test_db.pool());

    // create clients
    let clients = vec![
        NewClient {
            hub_id: 1,
            name: "Alice".into(),
            email: "alice@example.com".into(),
            phone: "111".into(),
            address: "Addr1".into(),
        },
        NewClient {
            hub_id: 1,
            name: "Bob".into(),
            email: "bob@example.com".into(),
            phone: "222".into(),
            address: "Addr2".into(),
        },
    ];
    client_repo.create(&clients).unwrap();
    let (_, stored_clients) = client_repo.list(ClientListQuery::new(1)).unwrap();
    let client_ids: Vec<i32> = stored_clients.iter().map(|c| c.id).collect();

    // create or update manager
    let manager = manager_repo
        .create_or_update(&NewManager {
            hub_id: 1,
            name: "Manager",
            email: "m@example.com",
        })
        .unwrap();
    assert!(manager.id > 0);

    let updated = manager_repo
        .create_or_update(&NewManager {
            hub_id: 1,
            name: "Updated",
            email: "m@example.com",
        })
        .unwrap();
    assert_eq!(updated.id, manager.id);
    assert_eq!(updated.name, "Updated");

    let by_id = manager_repo.get_by_id(manager.id).unwrap().unwrap();
    assert_eq!(by_id.name, "Updated");

    let by_email = manager_repo
        .get_by_email("m@example.com", 1)
        .unwrap()
        .unwrap();
    assert_eq!(by_email.id, manager.id);

    // assign clients to manager
    manager_repo
        .assign_clients(manager.id, &client_ids)
        .unwrap();

    let managers_with_clients = manager_repo.list(1).unwrap();
    assert_eq!(managers_with_clients.len(), 1);
    assert_eq!(managers_with_clients[0].0.id, manager.id);
    assert_eq!(managers_with_clients[0].1.len(), client_ids.len());

    let client_id = client_ids[0];
    let managers = client_repo.list_managers(client_id).unwrap();
    assert_eq!(managers.len(), 1);
    assert_eq!(managers[0].id, manager.id);
    assert!(
        client_repo
            .check_manager_assigned(client_id, "m@example.com")
            .unwrap()
    );
}
