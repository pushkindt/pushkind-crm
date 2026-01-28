//! Background worker consuming ZeroMQ notifications and recording CRM client events.

use std::env;

use config::Config;
use dotenvy::dotenv;
use pushkind_common::{
    db::establish_connection_pool,
    repository::errors::{RepositoryError, RepositoryResult},
};
use pushkind_emailer::models::zmq::{ZMQReplyMessage, ZMQSendEmailMessage, ZMQUnsubscribeMessage};
use pushkind_todo::dto::zmq::ZmqTask;
use serde_json::json;

use pushkind_crm::models::config::ServerConfig;
use pushkind_crm::repository::{
    ClientEventReader, ClientEventWriter, ClientReader, ClientWriter, DieselRepository,
    ManagerWriter,
};
use pushkind_crm::{
    domain::{
        client::NewClient,
        client_event::{ClientEventType, NewClientEvent},
        manager::NewManager,
        types::{ClientEmail, ClientName, HubId, PhoneNumber, PublicId},
    },
    models::zmq::ZmqClientMessage,
};

fn process_email_event<R>(msg: ZMQSendEmailMessage, repo: R) -> RepositoryResult<()>
where
    R: ClientEventWriter + ManagerWriter + ClientReader + ClientEventReader,
{
    match msg {
        ZMQSendEmailMessage::NewEmail(boxed) => {
            let (user, new_email) = *boxed;
            log::info!("New email from user {user:?}, {:?}", new_email.subject);
            let manager_payload = NewManager::try_from(&user).map_err(RepositoryError::from)?;
            let manager = repo.create_or_update_manager(&manager_payload)?;

            for recipient in &new_email.recipients {
                let recipient_email =
                    ClientEmail::new(recipient.address.as_str()).map_err(RepositoryError::from)?;
                let client = match repo.get_client_by_email(&recipient_email, manager.hub_id)? {
                    Some(client) => client,
                    None => {
                        continue;
                    }
                };

                let new_event = NewClientEvent::new(
                    client.id,
                    manager.id,
                    ClientEventType::Email,
                    json!({
                        "text": new_email.subject.as_ref().map(|s| s.as_str()),
                    }),
                );

                if repo.client_event_exists(&new_event)? {
                    log::info!(
                        "Skipping duplicate email event for client {} and manager {}",
                        client.id,
                        manager.id
                    );
                    continue;
                }

                match repo.create_client_event(&new_event) {
                    Ok(_) => {
                        log::info!("Created client event for client {}", client.id);
                    }
                    Err(e) => {
                        log::error!("Error creating client event: {e}");
                    }
                }
            }
        }
        _ => {
            log::error!("Skipping unsupported email types");
        }
    }
    Ok(())
}

fn process_task_message<R>(task: ZmqTask, repo: R) -> RepositoryResult<()>
where
    R: ClientEventWriter + ClientEventReader + ClientReader + ManagerWriter,
{
    let ZmqTask {
        public_id: task_public_id,
        hub_id: task_hub_id,
        title,
        priority,
        status,
        client,
        assignee,
        description,
        track,
        author,
        ..
    } = task;

    let client_snapshot = match client {
        Some(client) => client,
        None => {
            log::info!("Skipping task {} without client snapshot", task_public_id);
            return Ok(());
        }
    };

    let hub_id = HubId::new(task_hub_id).map_err(RepositoryError::from)?;
    let public_id = match client_snapshot.public_id.parse::<PublicId>() {
        Ok(public_id) => public_id,
        Err(err) => {
            log::warn!(
                "Skipping task {} with invalid client public_id {}: {err}",
                task_public_id,
                client_snapshot.public_id
            );
            return Ok(());
        }
    };

    let client = match repo.get_client_by_public_id(public_id, hub_id)? {
        Some(client) => client,
        None => {
            log::info!(
                "Skipping task {}: no CRM client for hub {} public_id {}",
                task_public_id,
                hub_id,
                client_snapshot.public_id
            );
            return Ok(());
        }
    };

    let manager_payload = NewManager::try_new(task_hub_id, author.name, author.email, false)
        .map_err(RepositoryError::from)?;
    let manager = repo.create_or_update_manager(&manager_payload)?;

    let assignee = assignee.as_ref().map(|assignee| {
        json!({
            "name": assignee.name,
            "email": assignee.email,
        })
    });
    let priority: &'static str = priority.into();
    let status: &'static str = status.into();
    let event_data = json!({
        "public_id": task_public_id,
        "text": description.as_deref(),
        "subject": title,
        "track": track.as_deref(),
        "priority": priority,
        "status": status,
        "assignee": assignee,
    });

    let event = NewClientEvent::new(client.id, manager.id, ClientEventType::Task, event_data);

    if repo.client_event_exists(&event)? {
        log::info!(
            "Skipping duplicate task event for client {} and manager {}",
            client.id,
            manager.id
        );
        return Ok(());
    }

    let _event = repo.create_client_event(&event)?;
    Ok(())
}

fn process_reply_message<R>(reply: ZMQReplyMessage, repo: R) -> RepositoryResult<()>
where
    R: ClientEventWriter + ManagerWriter + ClientReader + ClientEventReader,
{
    log::info!("Reply from {} in hub#{}", reply.email, reply.hub_id);

    let hub_id = HubId::new(reply.hub_id).map_err(RepositoryError::from)?;
    let reply_email = ClientEmail::new(&reply.email).map_err(RepositoryError::from)?;
    match repo.get_client_by_email(&reply_email, hub_id)? {
        Some(client) => {
            let new_manager = NewManager::try_new(
                client.hub_id.get(),
                client.name.as_str().to_string(),
                reply.email.clone(),
                false,
            )
            .map_err(RepositoryError::from)?;
            let manager = repo.create_or_update_manager(&new_manager)?;
            let event = NewClientEvent::new(
                client.id,
                manager.id,
                ClientEventType::Reply,
                json!({
                    "subject": &reply.subject,
                    "text": ammonia::clean(&reply.message),
                }),
            );
            if repo.client_event_exists(&event)? {
                log::info!(
                    "Skipping duplicate reply event for client {} and manager {}",
                    client.id,
                    manager.id
                );
                return Ok(());
            }
            let _event = repo.create_client_event(&event)?;
        }
        None => return Ok(()),
    }
    Ok(())
}

fn process_unsubscribe_message<R>(message: ZMQUnsubscribeMessage, repo: R) -> RepositoryResult<()>
where
    R: ClientEventWriter + ManagerWriter + ClientReader + ClientEventReader,
{
    log::info!(
        "Unsubscribe notification for {} in hub#{}",
        message.email,
        message.hub_id
    );

    let hub_id = HubId::new(message.hub_id).map_err(RepositoryError::from)?;
    let message_email = ClientEmail::new(&message.email).map_err(RepositoryError::from)?;
    match repo.get_client_by_email(&message_email, hub_id)? {
        Some(client) => {
            let new_manager = NewManager::try_new(
                client.hub_id.get(),
                client.name.as_str().to_string(),
                message.email.clone(),
                false,
            )
            .map_err(RepositoryError::from)?;
            let manager = repo.create_or_update_manager(&new_manager)?;
            let event = NewClientEvent::new(
                client.id,
                manager.id,
                ClientEventType::Unsubscribed,
                json!({
                    "text": &message.reason,
                }),
            );

            if repo.client_event_exists(&event)? {
                log::info!(
                    "Skipping duplicate unsubscribe event for client {} and manager {}",
                    client.id,
                    manager.id
                );
                return Ok(());
            }

            let _event = repo.create_client_event(&event)?;
        }
        None => return Ok(()),
    }

    Ok(())
}

fn process_client_message<R>(message: ZmqClientMessage, repo: R) -> RepositoryResult<()>
where
    R: ClientWriter + ClientReader,
{
    let new_client = NewClient::new(
        HubId::new(message.hub_id)?,
        ClientName::new(&message.name)?,
        match message.email {
            Some(email) => Some(ClientEmail::new(&email)?),
            None => None,
        },
        match message.phone {
            Some(phone) => Some(PhoneNumber::new(&phone)?),
            None => None,
        },
        message.fields,
    );

    let inserted = repo.create_clients(&[new_client])?;
    log::info!(
        "Inserted {} client records via ZMQ payload, skipped existing ones",
        inserted
    );

    Ok(())
}

fn main() {
    dotenv().ok(); // Load .env file
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Select config profile (defaults to `local`).
    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "local".into());

    let settings = Config::builder()
        // Add `./config/default.yaml`
        .add_source(config::File::with_name("config/default"))
        // Add environment-specific overrides
        .add_source(config::File::with_name(&format!("config/{}", app_env)).required(false))
        // Add settings from the environment (with a prefix of APP)
        .add_source(config::Environment::with_prefix("APP"))
        .build();

    let settings = match settings {
        Ok(settings) => settings,
        Err(err) => {
            log::error!("Error loading settings: {}", err);
            std::process::exit(1);
        }
    };

    let server_config = match settings.try_deserialize::<ServerConfig>() {
        Ok(server_config) => server_config,
        Err(err) => {
            log::error!("Error loading server config: {}", err);
            std::process::exit(1);
        }
    };

    let context = zmq::Context::new();
    let responder = context.socket(zmq::SUB).expect("Cannot create zmq socket");
    responder
        .connect(&server_config.zmq_emailer_sub)
        .expect("Cannot connect to zmq port");
    responder.set_subscribe(b"").expect("SUBSCRIBE failed");

    let replier = context.socket(zmq::SUB).expect("Cannot create zmq socket");
    replier
        .connect(&server_config.zmq_replier_sub)
        .expect("Cannot connect to zmq port");
    replier.set_subscribe(b"").expect("SUBSCRIBE failed");

    let clients = context.socket(zmq::SUB).expect("Cannot create zmq socket");
    clients
        .connect(&server_config.zmq_clients_sub)
        .expect("Cannot connect to zmq port");
    clients.set_subscribe(b"").expect("SUBSCRIBE failed");

    let tasks = context.socket(zmq::SUB).expect("Cannot create zmq socket");
    tasks
        .connect(&server_config.zmq_tasks_sub)
        .expect("Cannot connect to zmq port");
    tasks.set_subscribe(b"").expect("SUBSCRIBE failed");

    let pool = match establish_connection_pool(&server_config.database_url) {
        Ok(pool) => pool,
        Err(e) => {
            log::error!("Failed to establish database connection: {e}");
            std::process::exit(1);
        }
    };

    let repo = DieselRepository::new(pool);

    log::info!("Starting event worker");

    let reply_repo = repo.clone();
    std::thread::spawn(move || {
        loop {
            let msg = replier.recv_bytes(0).unwrap();

            if let Ok(reply) = serde_json::from_slice::<ZMQReplyMessage>(&msg) {
                let repo = reply_repo.clone();
                if let Err(e) = process_reply_message(reply, repo) {
                    log::error!("Error processing reply message: {e}");
                }
                continue;
            }

            match serde_json::from_slice::<ZMQUnsubscribeMessage>(&msg) {
                Ok(unsubscribe) => {
                    let repo = reply_repo.clone();
                    if let Err(e) = process_unsubscribe_message(unsubscribe, repo) {
                        log::error!("Error processing unsubscribe message: {e}");
                    }
                }
                Err(e) => log::error!("Error receiving replier message: {e}"),
            }
        }
    });

    let client_repo = repo.clone();
    std::thread::spawn(move || {
        loop {
            let msg = clients.recv_bytes(0).unwrap();
            match serde_json::from_slice::<ZmqClientMessage>(&msg) {
                Ok(parsed) => {
                    if let Err(e) = process_client_message(parsed, client_repo.clone()) {
                        log::error!("Error processing client message: {e}");
                    }
                }
                Err(e) => log::error!("Error receiving client message: {e}"),
            }
        }
    });

    let task_repo = repo.clone();
    std::thread::spawn(move || {
        loop {
            let msg = tasks.recv_bytes(0).unwrap();
            match serde_json::from_slice::<ZmqTask>(&msg) {
                Ok(parsed) => {
                    if let Err(e) = process_task_message(parsed, task_repo.clone()) {
                        log::error!("Error processing task message: {e}");
                    }
                }
                Err(e) => log::error!("Error receiving task message: {e}"),
            }
        }
    });

    loop {
        let msg = responder.recv_bytes(0).unwrap();
        match serde_json::from_slice::<ZMQSendEmailMessage>(&msg) {
            Ok(parsed) => {
                let repo = repo.clone();
                if let Err(e) = process_email_event(parsed, repo) {
                    log::error!("Error processing email message: {e}");
                }
            }
            Err(e) => {
                log::error!("Error receiving message: {e}");
                continue;
            }
        }
    }
}

#[cfg(all(test, feature = "test-mocks"))]
mod tests {
    use super::*;
    use chrono::Utc;
    use pushkind_crm::domain::client::Client;
    use pushkind_crm::domain::client_event::ClientEvent;
    use pushkind_crm::domain::manager::Manager;
    use pushkind_crm::domain::types::{ClientEventId, ClientId, ClientName, HubId, PublicId};
    use pushkind_crm::repository::mock::MockRepository;
    use pushkind_todo::domain::task::{TaskPriority, TaskStatus};
    use pushkind_todo::dto::zmq::{ZmqTask, ZmqTaskAssignee, ZmqTaskAuthor, ZmqTaskClient};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct TestState {
        clients: Arc<Mutex<HashMap<ClientId, Client>>>,
        next_id: Arc<Mutex<i32>>,
    }

    impl Default for TestState {
        fn default() -> Self {
            Self {
                clients: Arc::new(Mutex::new(HashMap::new())),
                next_id: Arc::new(Mutex::new(1)),
            }
        }
    }

    impl TestState {
        fn snapshot(&self) -> HashMap<ClientId, Client> {
            self.clients.lock().expect("lock poisoned").clone()
        }
    }

    fn build_repo(state: TestState) -> MockRepository {
        let mut repo = MockRepository::new();
        let clients = state.clients.clone();
        let next_id = state.next_id.clone();

        repo.expect_create_clients()
            .times(1)
            .returning(move |new_clients| {
                let mut count = 0;
                let mut clients = clients.lock().expect("lock poisoned");
                let mut next_id = next_id.lock().expect("lock poisoned");

                for new in new_clients {
                    let now = Utc::now().naive_utc();

                    if let Some(email) = new.email.as_ref()
                        && clients.values().any(|client| {
                            client.hub_id == new.hub_id && client.email.as_ref() == Some(email)
                        })
                    {
                        continue;
                    }

                    let id = ClientId::new(*next_id).expect("valid client id");
                    *next_id += 1;
                    let client = Client {
                        id,
                        public_id: Some(PublicId::new()),
                        hub_id: new.hub_id,
                        name: new.name.clone(),
                        email: new.email.clone(),
                        phone: new.phone.clone(),
                        created_at: now,
                        updated_at: now,
                        fields: new.fields.clone(),
                    };
                    clients.insert(id, client);
                    count += 1;
                }

                Ok(count)
            });

        repo
    }

    #[test]
    fn processes_new_client_payloads() {
        let state = TestState::default();
        let message_alice = ZmqClientMessage {
            hub_id: 1,
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
            phone: None,
            fields: None,
        };
        process_client_message(message_alice, build_repo(state.clone()))
            .expect("processing failed");

        let message_bob = ZmqClientMessage {
            hub_id: 2,
            name: "Bob".to_string(),
            email: Some("bob@example.com".to_string()),
            phone: Some("+1 (415) 555-2671".to_string()),
            fields: None,
        };
        process_client_message(message_bob, build_repo(state.clone())).expect("processing failed");

        let snapshot = state.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert!(snapshot.values().any(|c| c.name.as_str() == "Alice"));
        assert!(snapshot.values().any(|c| c.name.as_str() == "Bob"));
    }

    #[test]
    fn does_not_update_existing_clients_by_email() {
        let state = TestState::default();
        let create_message = ZmqClientMessage {
            hub_id: 1,
            name: "Initial".to_string(),
            email: Some("initial@example.com".to_string()),
            phone: None,
            fields: None,
        };
        process_client_message(create_message, build_repo(state.clone())).expect("insert failed");

        let inserted_id = state.snapshot().values().next().expect("client missing").id;

        let update_message = ZmqClientMessage {
            hub_id: 1,
            name: "Updated".to_string(),
            email: Some("initial@example.com".to_string()),
            phone: Some("+1 (415) 555-2671".to_string()),
            fields: None,
        };

        process_client_message(update_message, build_repo(state.clone())).expect("update failed");

        let snapshot = state.snapshot();
        assert_eq!(snapshot.len(), 1);
        let updated = snapshot.get(&inserted_id).expect("client missing");
        assert_eq!(updated.name.as_str(), "Initial");
        assert_eq!(updated.phone.as_ref().map(|phone| phone.as_str()), None);
    }

    #[test]
    fn creates_new_when_email_not_found() {
        let state = TestState::default();
        let message = ZmqClientMessage {
            hub_id: 9,
            name: "Fallback".to_string(),
            email: Some("fallback@example.com".to_string()),
            phone: None,
            fields: None,
        };

        process_client_message(message, build_repo(state.clone())).expect("processing failed");

        let snapshot = state.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot.values().next().unwrap().hub_id.get(), 9);
    }

    #[test]
    fn creates_new_when_email_missing() {
        let state = TestState::default();
        let message = ZmqClientMessage {
            hub_id: 3,
            name: "No Email".to_string(),
            email: None,
            phone: None,
            fields: None,
        };

        process_client_message(message, build_repo(state.clone())).expect("processing failed");

        let snapshot = state.snapshot();
        assert_eq!(snapshot.len(), 1);
        let client = snapshot.values().next().unwrap();
        assert_eq!(client.name.as_str(), "No Email");
        assert_eq!(client.hub_id.get(), 3);
    }

    #[test]
    fn process_task_message_creates_event_for_matching_client() {
        let mut repo = MockRepository::new();
        let hub_id = HubId::new(1).expect("valid hub id");
        let public_id = PublicId::new();
        let client = Client {
            id: ClientId::new(10).expect("valid client id"),
            public_id: Some(public_id),
            hub_id,
            name: ClientName::new("Client").expect("valid name"),
            email: None,
            phone: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            fields: None,
        };

        let manager = Manager::try_new(
            5,
            hub_id.get(),
            "Manager".to_string(),
            "manager@example.com".to_string(),
            false,
        )
        .expect("valid manager");

        repo.expect_get_client_by_public_id()
            .times(1)
            .returning(move |pid, hid| {
                if pid == public_id && hid == hub_id {
                    Ok(Some(client.clone()))
                } else {
                    Ok(None)
                }
            });

        repo.expect_create_or_update_manager()
            .times(1)
            .returning(move |_| Ok(manager.clone()));

        repo.expect_client_event_exists()
            .times(1)
            .returning(|_| Ok(false));

        repo.expect_create_client_event()
            .times(1)
            .withf(|event| {
                event.event_type == ClientEventType::Task
                    && event.event_data["public_id"] == json!("task-1")
                    && event.event_data["subject"] == json!("Task title")
                    && event.event_data["text"] == json!("Task description")
                    && event.event_data["track"] == json!("Track A")
                    && event.event_data["priority"] == json!("High")
                    && event.event_data["status"] == json!("Pending")
                    && event.event_data["assignee"]["name"] == json!("Assignee")
                    && event.event_data["assignee"]["email"] == json!("assignee@example.com")
            })
            .returning(move |event| {
                Ok(ClientEvent::new(
                    ClientEventId::new(1).expect("valid event id"),
                    event.client_id,
                    event.manager_id,
                    event.event_type.clone(),
                    event.event_data.clone(),
                    Utc::now().naive_utc(),
                ))
            });

        let task = ZmqTask {
            public_id: "task-1".to_string(),
            hub_id: hub_id.get(),
            title: "Task title".to_string(),
            priority: TaskPriority::High,
            status: TaskStatus::Pending,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            due_date: None,
            completed_at: None,
            author: ZmqTaskAuthor {
                name: "Manager".to_string(),
                email: "manager@example.com".to_string(),
            },
            client: Some(ZmqTaskClient {
                name: "Client".to_string(),
                public_id: public_id.to_string(),
            }),
            assignee: Some(ZmqTaskAssignee {
                name: "Assignee".to_string(),
                email: "assignee@example.com".to_string(),
            }),
            description: Some("Task description".to_string()),
            track: Some("Track A".to_string()),
        };

        process_task_message(task, repo).expect("task processing failed");
    }
}
