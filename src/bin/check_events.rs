use std::env;

use chrono::Utc;
use dotenvy::dotenv;
use pushkind_common::models::emailer::zmq::{ZMQReplyMessage, ZMQSendEmailMessage};
use pushkind_common::{db::establish_connection_pool, repository::errors::RepositoryResult};

use pushkind_crm::domain::{
    client_event::{ClientEventType, NewClientEvent},
    manager::NewManager,
};
use pushkind_crm::repository::{
    ClientEventReader, ClientEventWriter, ClientReader, DieselRepository, ManagerWriter,
};
use serde_json::json;

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
                    client_id: client.id,
                    event_type: ClientEventType::Email,
                    manager_id: manager.id,
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
            let new_manager =
                NewManager::new(client.hub_id, client.name.clone(), reply.email.clone());
            let manager = repo.create_or_update_manager(&new_manager)?;
            let event = NewClientEvent {
                client_id: client.id,
                manager_id: manager.id,
                event_type: ClientEventType::Reply,
                event_data: json!({
                    "text": reply.message,
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

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok(); // Load .env file

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "app.db".to_string());

    let zmq_address =
        env::var("ZMQ_EMAILER_SUB").unwrap_or_else(|_| "tcp://127.0.0.1:5558".to_string());
    let replier_address =
        env::var("ZMQ_REPLIER_SUB").unwrap_or_else(|_| "tcp://127.0.0.1:5560".to_string());
    let context = zmq::Context::new();
    let responder = context.socket(zmq::SUB).expect("Cannot create zmq socket");
    responder
        .connect(&zmq_address)
        .expect("Cannot connect to zmq port");
    responder.set_subscribe(b"").expect("SUBSCRIBE failed");

    let replier = context.socket(zmq::SUB).expect("Cannot create zmq socket");
    replier
        .connect(&replier_address)
        .expect("Cannot connect to zmq port");
    replier.set_subscribe(b"").expect("SUBSCRIBE failed");

    let pool = match establish_connection_pool(&database_url) {
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
            match serde_json::from_slice::<ZMQReplyMessage>(&msg) {
                Ok(reply) => {
                    let repo = reply_repo.clone();
                    if let Err(e) = process_reply_message(reply, repo) {
                        log::error!("Error processing reply message: {e}");
                    }
                }
                Err(e) => log::error!("Error receiving reply message: {e}"),
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
