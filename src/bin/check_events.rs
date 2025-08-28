use std::env;

use chrono::Utc;
use dotenvy::dotenv;
use pushkind_common::models::zmq::emailer::ZMQSendEmailMessage;
use pushkind_common::{db::establish_connection_pool, repository::errors::RepositoryResult};

use pushkind_crm::domain::client_event::{ClientEventType, NewClientEvent};
use pushkind_crm::repository::{ClientEventWriter, ClientReader, DieselRepository, ManagerWriter};
use serde_json::json;

async fn process_email_event<R>(msg: ZMQSendEmailMessage, repo: R) -> RepositoryResult<()>
where
    R: ClientEventWriter + ManagerWriter + ClientReader,
{
    match msg {
        ZMQSendEmailMessage::NewEmail((user, new_email)) => {
            log::info!("New email from user {user:?}, {:?}", new_email.subject);
            let manager = repo.create_or_update_manager(&(&user).into())?;

            for recipient in &new_email.recipients {
                let client = match repo.get_client_by_email(&recipient.address, manager.hub_id)? {
                    Some(client) => client,
                    None => {
                        continue;
                    }
                };

                let event = NewClientEvent {
                    client_id: client.id,
                    event_type: ClientEventType::Email,
                    manager_id: manager.id,
                    created_at: Utc::now().naive_utc(),
                    event_data: json!({
                        "text": new_email.subject.as_deref().unwrap_or_default(),
                    }),
                };

                match repo.create_client_event(&event) {
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

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok(); // Load .env file

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "app.db".to_string());

    let zmq_address =
        env::var("ZMQ_EMAILER_SUB").unwrap_or_else(|_| "tcp://127.0.0.1:5558".to_string());
    let context = zmq::Context::new();
    let responder = context.socket(zmq::SUB).expect("Cannot create zmq socket");
    responder
        .connect(&zmq_address)
        .expect("Cannot connect to zmq port");
    responder.set_subscribe(b"").expect("SUBSCRIBE failed");

    let pool = match establish_connection_pool(&database_url) {
        Ok(pool) => pool,
        Err(e) => {
            log::error!("Failed to establish database connection: {e}");
            std::process::exit(1);
        }
    };

    let repo = DieselRepository::new(pool);

    log::info!("Starting event worker");

    loop {
        let msg = responder.recv_bytes(0).unwrap();
        match serde_json::from_slice::<ZMQSendEmailMessage>(&msg) {
            Ok(parsed) => {
                let repo = repo.clone();

                tokio::spawn(async move {
                    if let Err(e) = process_email_event(parsed, repo).await {
                        log::error!("Error processing email message: {e}");
                    }
                });
            }
            Err(e) => {
                log::error!("Error receiving message: {e}");
                continue;
            }
        }
    }
}
