//! Background worker consuming ZeroMQ notifications and recording CRM client events.

use std::{collections::BTreeMap, env};

use chrono::Utc;
use config::Config;
use dotenvy::dotenv;
use pushkind_common::models::emailer::zmq::{
    ZMQReplyMessage, ZMQSendEmailMessage, ZMQUnsubscribeMessage,
};
use pushkind_common::{
    db::establish_connection_pool,
    repository::errors::{RepositoryError, RepositoryResult},
};
use serde::Deserialize;
use serde_json::json;

use pushkind_crm::domain::{
    client::{NewClient, UpdateClient},
    client_event::{ClientEventType, NewClientEvent},
    manager::NewManager,
    types::{ClientId, ManagerId},
};
use pushkind_crm::models::config::ServerConfig;
use pushkind_crm::repository::{
    ClientEventReader, ClientEventWriter, ClientReader, ClientWriter, DieselRepository,
    ManagerWriter,
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
            let manager = repo.create_or_update_manager(&(&user).into())?;

            for recipient in &new_email.recipients {
                let client = match repo.get_client_by_email(&recipient.address, manager.hub_id)? {
                    Some(client) => client,
                    None => {
                        continue;
                    }
                };

                let new_event = NewClientEvent {
                    client_id: ClientId::try_from(client.id)?,
                    event_type: ClientEventType::Email,
                    manager_id: ManagerId::try_from(manager.id)?,
                    created_at: Utc::now().naive_utc(),
                    event_data: json!({
                        "text": new_email.subject.as_deref().unwrap_or_default(),
                    }),
                };

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

    match repo.get_client_by_email(&reply.email, reply.hub_id)? {
        Some(client) => {
            let new_manager = NewManager::new(
                client.hub_id,
                client.name.clone(),
                reply.email.clone(),
                false,
            );
            let manager = repo.create_or_update_manager(&new_manager)?;
            let event = NewClientEvent {
                client_id: ClientId::try_from(client.id)?,
                manager_id: ManagerId::try_from(manager.id)?,
                event_type: ClientEventType::Reply,
                event_data: json!({
                    "subject": &reply.subject,
                    "text": ammonia::clean(&reply.message),
                }),
                created_at: Utc::now().naive_utc(),
            };
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

    match repo.get_client_by_email(&message.email, message.hub_id)? {
        Some(client) => {
            let new_manager = NewManager::new(
                client.hub_id,
                client.name.clone(),
                message.email.clone(),
                false,
            );
            let manager = repo.create_or_update_manager(&new_manager)?;
            let event = NewClientEvent {
                client_id: ClientId::try_from(client.id)?,
                manager_id: ManagerId::try_from(manager.id)?,
                event_type: ClientEventType::Unsubscribed,
                event_data: json!({
                    "text": &message.reason,
                }),
                created_at: Utc::now().naive_utc(),
            };

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

#[derive(Debug, Deserialize)]
struct ZmqClientPayload {
    hub_id: i32,
    name: String,
    email: Option<String>,
    phone: Option<String>,
    #[serde(default)]
    fields: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ZmqClientMessage {
    Batch { clients: Vec<ZmqClientPayload> },
    Single { client: ZmqClientPayload },
    Array(Vec<ZmqClientPayload>),
    Plain(ZmqClientPayload),
}

impl ZmqClientMessage {
    fn into_payloads(self) -> Vec<ZmqClientPayload> {
        match self {
            Self::Batch { clients } => clients,
            Self::Single { client } => vec![client],
            Self::Array(clients) => clients,
            Self::Plain(client) => vec![client],
        }
    }
}

fn process_client_message<R>(message: ZmqClientMessage, repo: R) -> RepositoryResult<()>
where
    R: ClientWriter + ClientReader,
{
    let payloads = message.into_payloads();

    if payloads.is_empty() {
        log::info!("Received empty client payload; skipping");
        return Ok(());
    }

    let mut new_clients: Vec<NewClient> = Vec::new();

    for payload in payloads {
        let ZmqClientPayload {
            hub_id,
            name,
            email,
            phone,
            fields,
        } = payload;

        if let Some(email_lookup) = email.clone() {
            match repo.get_client_by_email(&email_lookup, hub_id)? {
                Some(existing) => {
                    let updates = UpdateClient::new(
                        name.clone(),
                        email.clone(),
                        phone.clone(),
                        fields.clone(),
                    );

                    match repo.update_client(existing.id, &updates) {
                        Ok(client) => {
                            log::info!(
                                "Updated client {} in hub {} via ZMQ payload",
                                client.id,
                                client.hub_id
                            );
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to update client {} in hub {}: {}",
                                existing.id,
                                hub_id,
                                e
                            );
                            return Err(e);
                        }
                    }

                    continue;
                }
                None => {
                    log::info!(
                        "Creating new client in hub {} for email {}",
                        hub_id,
                        email_lookup
                    );
                }
            }
        }

        new_clients.push(NewClient::new(hub_id, name, email, phone, fields));
    }

    if !new_clients.is_empty() {
        let inserted = repo.create_clients(&new_clients)?;
        log::info!(
            "Inserted or updated {} client records via ZMQ payload",
            inserted
        );
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use pushkind_common::repository::errors::RepositoryError;
    use pushkind_crm::domain::client::Client;
    use pushkind_crm::domain::manager::Manager;
    use pushkind_crm::repository::ClientListQuery;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct TestRepo {
        clients: Arc<Mutex<HashMap<i32, Client>>>,
        next_id: Arc<Mutex<i32>>,
    }

    impl TestRepo {
        fn new() -> Self {
            Self::default()
        }

        fn snapshot(&self) -> HashMap<i32, Client> {
            self.clients.lock().expect("lock poisoned").clone()
        }
    }

    impl ClientReader for TestRepo {
        fn list_available_fields(&self, _hub_id: i32) -> RepositoryResult<Vec<String>> {
            Ok(Vec::new())
        }

        fn get_client_by_id(&self, id: i32, _hub_id: i32) -> RepositoryResult<Option<Client>> {
            Ok(self.snapshot().get(&id).cloned())
        }

        fn get_client_by_email(
            &self,
            email: &str,
            hub_id: i32,
        ) -> RepositoryResult<Option<Client>> {
            Ok(self
                .snapshot()
                .values()
                .find(|c| c.hub_id == hub_id && c.email.as_deref() == Some(email))
                .cloned())
        }

        fn list_clients(&self, _query: ClientListQuery) -> RepositoryResult<(usize, Vec<Client>)> {
            Ok((0, Vec::new()))
        }

        fn search_clients(
            &self,
            _query: ClientListQuery,
        ) -> RepositoryResult<(usize, Vec<Client>)> {
            Ok((0, Vec::new()))
        }

        fn list_managers(&self, _id: i32) -> RepositoryResult<Vec<Manager>> {
            Ok(Vec::new())
        }

        fn check_client_assigned_to_manager(
            &self,
            _client_id: i32,
            _manager_email: &str,
        ) -> RepositoryResult<bool> {
            Ok(false)
        }
    }

    impl ClientWriter for TestRepo {
        fn create_clients(&self, new_clients: &[NewClient]) -> RepositoryResult<usize> {
            let mut count = 0;
            let mut clients = self.clients.lock().expect("lock poisoned");
            let mut next_id = self.next_id.lock().expect("lock poisoned");

            for new in new_clients {
                let id = *next_id;
                *next_id += 1;
                let timestamp = Utc::now().naive_utc();
                let client = Client {
                    id,
                    hub_id: new.hub_id,
                    name: new.name.clone(),
                    email: new.email.clone(),
                    phone: new.phone.clone(),
                    created_at: timestamp,
                    updated_at: timestamp,
                    fields: new.fields.clone(),
                };
                clients.insert(id, client);
                count += 1;
            }

            Ok(count)
        }

        fn update_client(
            &self,
            client_id: i32,
            updates: &UpdateClient,
        ) -> RepositoryResult<Client> {
            let mut clients = self.clients.lock().expect("lock poisoned");
            let Some(existing) = clients.get_mut(&client_id) else {
                return Err(RepositoryError::NotFound);
            };

            existing.name = updates.name.clone();
            existing.email = updates.email.clone();
            existing.phone = updates.phone.clone();
            existing.fields = updates.fields.clone();
            existing.updated_at = Utc::now().naive_utc();

            Ok(existing.clone())
        }

        fn delete_client(&self, client_id: i32) -> RepositoryResult<()> {
            self.clients
                .lock()
                .expect("lock poisoned")
                .remove(&client_id);
            Ok(())
        }
    }

    #[test]
    fn processes_new_client_payloads() {
        let repo = TestRepo::new();
        let message = ZmqClientMessage::Batch {
            clients: vec![
                ZmqClientPayload {
                    hub_id: 1,
                    name: "Alice".to_string(),
                    email: Some("alice@example.com".to_string()),
                    phone: None,
                    fields: None,
                },
                ZmqClientPayload {
                    hub_id: 2,
                    name: "Bob".to_string(),
                    email: Some("bob@example.com".to_string()),
                    phone: Some("123".to_string()),
                    fields: None,
                },
            ],
        };

        process_client_message(message, repo.clone()).expect("processing failed");

        let snapshot = repo.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert!(snapshot.values().any(|c| c.name == "Alice"));
        assert!(snapshot.values().any(|c| c.name == "Bob"));
    }

    #[test]
    fn updates_existing_clients_by_email() {
        let repo = TestRepo::new();
        let create_message = ZmqClientMessage::Plain(ZmqClientPayload {
            hub_id: 1,
            name: "Initial".to_string(),
            email: Some("initial@example.com".to_string()),
            phone: None,
            fields: None,
        });
        process_client_message(create_message, repo.clone()).expect("insert failed");

        let inserted_id = repo.snapshot().values().next().expect("client missing").id;

        let update_message = ZmqClientMessage::Plain(ZmqClientPayload {
            hub_id: 1,
            name: "Updated".to_string(),
            email: Some("initial@example.com".to_string()),
            phone: Some("+1 (415) 555-2671".to_string()),
            fields: None,
        });

        process_client_message(update_message, repo.clone()).expect("update failed");

        let snapshot = repo.snapshot();
        assert_eq!(snapshot.len(), 1);
        let updated = snapshot.get(&inserted_id).expect("client missing");
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.phone.as_deref(), Some("+14155552671"));
    }

    #[test]
    fn creates_new_when_email_not_found() {
        let repo = TestRepo::new();
        let message = ZmqClientMessage::Plain(ZmqClientPayload {
            hub_id: 9,
            name: "Fallback".to_string(),
            email: Some("fallback@example.com".to_string()),
            phone: None,
            fields: None,
        });

        process_client_message(message, repo.clone()).expect("processing failed");

        let snapshot = repo.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot.values().next().unwrap().hub_id, 9);
    }

    #[test]
    fn creates_new_when_email_missing() {
        let repo = TestRepo::new();
        let message = ZmqClientMessage::Plain(ZmqClientPayload {
            hub_id: 3,
            name: "No Email".to_string(),
            email: None,
            phone: None,
            fields: None,
        });

        process_client_message(message, repo.clone()).expect("processing failed");

        let snapshot = repo.snapshot();
        assert_eq!(snapshot.len(), 1);
        let client = snapshot.values().next().unwrap();
        assert_eq!(client.name, "No Email");
        assert_eq!(client.hub_id, 3);
    }
}
