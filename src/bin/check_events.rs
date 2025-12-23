//! Background worker consuming ZeroMQ notifications and recording CRM client events.

use std::env;

use config::Config;
use dotenvy::dotenv;
use pushkind_common::{
    db::establish_connection_pool,
    repository::errors::{RepositoryError, RepositoryResult},
};
use pushkind_emailer::models::zmq::{ZMQReplyMessage, ZMQSendEmailMessage, ZMQUnsubscribeMessage};
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
        types::{ClientEmail, ClientName, HubId, PhoneNumber},
    },
    models::zmq::ZmqClientMessage,
};

fn is_duplicate_event<R>(repo: &R, new_event: &NewClientEvent) -> RepositoryResult<bool>
where
    R: ClientEventReader,
{
    repo.client_event_exists(new_event)
}

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

                if is_duplicate_event(&repo, &new_event)? {
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
            if is_duplicate_event(&repo, &event)? {
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

            if is_duplicate_event(&repo, &event)? {
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
        "Inserted or updated {} client records via ZMQ payload",
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
    use pushkind_crm::domain::types::ClientId;
    use pushkind_crm::repository::mock::MockRepository;
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

                    let mut updated_existing = false;
                    if let Some(email) = new.email.as_ref()
                        && let Some(existing) = clients.values_mut().find(|client| {
                            client.hub_id == new.hub_id && client.email.as_ref() == Some(email)
                        })
                    {
                        existing.name = new.name.clone();
                        existing.phone = new.phone.clone();
                        existing.fields = new.fields.clone();
                        existing.updated_at = now;
                        updated_existing = true;
                    }

                    if updated_existing {
                        count += 1;
                        continue;
                    }

                    let id = ClientId::new(*next_id).expect("valid client id");
                    *next_id += 1;
                    let client = Client {
                        id,
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
    fn updates_existing_clients_by_email() {
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
        assert_eq!(updated.name.as_str(), "Updated");
        assert_eq!(
            updated.phone.as_ref().map(|phone| phone.as_str()),
            Some("+14155552671")
        );
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
}
