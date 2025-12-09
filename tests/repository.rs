use std::collections::BTreeMap;

use chrono::Utc;
use pushkind_crm::domain::client::{NewClient, UpdateClient};
use pushkind_crm::domain::client_event::{ClientEventType, NewClientEvent};
use pushkind_crm::domain::manager::NewManager;
use pushkind_crm::domain::types::{ClientId, ManagerId};
use pushkind_crm::repository::{ClientEventListQuery, ClientEventReader, ClientEventWriter};
use pushkind_crm::repository::{ClientListQuery, ClientReader, ClientWriter};
use pushkind_crm::repository::{DieselRepository, ManagerReader, ManagerWriter};
use serde_json::json;

mod common;

#[test]
fn test_client_repository_crud() {
    let test_db = common::TestDb::new();
    let client_repo = DieselRepository::new(test_db.pool());
    let c1 = NewClient::new(
        1,
        "Alice".into(),
        Some("alice@example.com".into()),
        Some("111".into()),
        None,
    );
    let c2 = NewClient::new(
        1,
        "Bob".into(),
        Some("bob@example.com".into()),
        Some("222".into()),
        None,
    );

    assert_eq!(
        client_repo
            .create_clients(&[c1.clone(), c2.clone()])
            .unwrap(),
        2
    );

    let (total, mut items) = client_repo.list_clients(ClientListQuery::new(1)).unwrap();
    assert_eq!(total, 2);
    assert_eq!(items.len(), 2);
    items.sort_by(|a, b| a.name.cmp(&b.name));
    let mut alice = items[0].clone();
    let mut bob = items[1].clone();

    let (search_total, search_items) = client_repo
        .search_clients(ClientListQuery::new(1).search("Bob"))
        .unwrap();
    assert_eq!(search_total, 1);
    assert_eq!(search_items[0].name, "Bob");

    alice = client_repo
        .update_client(
            alice.id,
            &UpdateClient::new(
                alice.name.clone(),
                alice.email.clone(),
                alice.phone.clone(),
                Some(BTreeMap::from([("vip".to_string(), "true".to_string())])),
            ),
        )
        .unwrap();
    assert_eq!(
        alice.fields,
        Some(BTreeMap::from([("vip".to_string(), "true".to_string())]))
    );

    bob = client_repo
        .update_client(
            bob.id,
            &UpdateClient::new(
                "Bobby".to_string(),
                bob.email.clone(),
                bob.phone.clone(),
                Some(BTreeMap::new()),
            ),
        )
        .unwrap();
    assert_eq!(bob.name, "Bobby");

    client_repo.delete_client(alice.id).unwrap();
    assert!(client_repo.get_client_by_id(alice.id, 1).unwrap().is_none());

    let (total_after, items_after) = client_repo.list_clients(ClientListQuery::new(1)).unwrap();
    assert_eq!(total_after, 1);
    assert_eq!(items_after[0].name, "Bobby");
}

#[test]
fn test_client_event_repository_crud() {
    let test_db = common::TestDb::new();
    let client_repo = DieselRepository::new(test_db.pool());
    let manager_repo = DieselRepository::new(test_db.pool());
    let client = {
        let new_client = NewClient::new(
            1,
            "Alice".into(),
            Some("alice@example.com".into()),
            Some("111".into()),
            None,
        );
        client_repo.create_clients(&[new_client]).unwrap();
        client_repo
            .list_clients(ClientListQuery::new(1))
            .unwrap()
            .1
            .remove(0)
    };
    let manager = manager_repo
        .create_or_update_manager(&NewManager::new(
            1,
            "Manager".to_string(),
            "m@example.com".to_string(),
            true,
        ))
        .unwrap();

    let client_event_repo = DieselRepository::new(test_db.pool());

    let new_event = NewClientEvent {
        client_id: ClientId::try_from(client.id).unwrap(),
        manager_id: ManagerId::try_from(manager.id).unwrap(),
        event_type: ClientEventType::Comment,
        event_data: json!({"text": "hello"}),
        created_at: Utc::now().naive_utc(),
    };
    let created = client_event_repo.create_client_event(&new_event).unwrap();
    assert_eq!(created.event_type, ClientEventType::Comment);

    let duplicate_attempt = NewClientEvent {
        created_at: Utc::now().naive_utc(),
        ..new_event.clone()
    };
    let duplicate = client_event_repo
        .create_client_event(&duplicate_attempt)
        .unwrap();
    assert_ne!(duplicate.id, created.id);

    let (total_after_duplicate, events_after_duplicate) = client_event_repo
        .list_client_events(ClientEventListQuery::new(client.id))
        .unwrap();
    assert_eq!(total_after_duplicate, 2);
    assert_eq!(events_after_duplicate.len(), 2);

    let _ = client_event_repo
        .create_client_event(&NewClientEvent {
            client_id: ClientId::try_from(client.id).unwrap(),
            manager_id: ManagerId::try_from(manager.id).unwrap(),
            event_type: ClientEventType::Call,
            event_data: json!({}),
            created_at: Utc::now().naive_utc(),
        })
        .unwrap();

    let (total, events) = client_event_repo
        .list_client_events(ClientEventListQuery::new(client.id))
        .unwrap();
    assert_eq!(total, 3);
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].1.id, manager.id);

    let (total_comment, comments) = client_event_repo
        .list_client_events(
            ClientEventListQuery::new(client.id).event_type(ClientEventType::Comment),
        )
        .unwrap();
    assert_eq!(total_comment, 2);
    assert!(
        comments
            .iter()
            .all(|(event, _)| event.event_type == ClientEventType::Comment)
    );
}

#[test]
fn test_manager_repository_crud() {
    let test_db = common::TestDb::new();
    let client_repo = DieselRepository::new(test_db.pool());
    let manager_repo = DieselRepository::new(test_db.pool());

    // create clients
    let clients = vec![
        NewClient::new(
            1,
            "Alice".into(),
            Some("alice@example.com".into()),
            Some("111".into()),
            None,
        ),
        NewClient::new(
            1,
            "Bob".into(),
            Some("bob@example.com".into()),
            Some("222".into()),
            None,
        ),
    ];
    client_repo.create_clients(&clients).unwrap();
    let (_, stored_clients) = client_repo.list_clients(ClientListQuery::new(1)).unwrap();
    let client_ids: Vec<i32> = stored_clients.iter().map(|c| c.id).collect();

    // create or update manager
    let manager = manager_repo
        .create_or_update_manager(&NewManager::new(
            1,
            "Manager".to_string(),
            "m@example.com".to_string(),
            true,
        ))
        .unwrap();
    assert!(manager.id > 0);

    let updated = manager_repo
        .create_or_update_manager(&NewManager::new(
            1,
            "Updated".to_string(),
            "m@example.com".to_string(),
            true,
        ))
        .unwrap();
    assert_eq!(updated.id, manager.id);
    assert_eq!(updated.name, "Updated");

    let preserved = manager_repo
        .create_or_update_manager(&NewManager::new(
            1,
            "Updated".to_string(),
            "m@example.com".to_string(),
            false,
        ))
        .unwrap();
    assert!(preserved.is_user);

    let by_id = manager_repo
        .get_manager_by_id(manager.id, 1)
        .unwrap()
        .unwrap();
    assert_eq!(by_id.name, "Updated");

    let by_email = manager_repo
        .get_manager_by_email("m@example.com", 1)
        .unwrap()
        .unwrap();
    assert_eq!(by_email.id, manager.id);

    // assign clients to manager
    manager_repo
        .assign_clients_to_manager(manager.id, &client_ids)
        .unwrap();

    let managers_with_clients = manager_repo.list_managers_with_clients(1).unwrap();
    assert_eq!(managers_with_clients.len(), 1);
    assert_eq!(managers_with_clients[0].0.id, manager.id);
    assert_eq!(managers_with_clients[0].1.len(), client_ids.len());

    let client_id = client_ids[0];
    let managers = client_repo.list_managers(client_id).unwrap();
    assert_eq!(managers.len(), 1);
    assert_eq!(managers[0].id, manager.id);
    assert!(
        client_repo
            .check_client_assigned_to_manager(client_id, "m@example.com")
            .unwrap()
    );
}
