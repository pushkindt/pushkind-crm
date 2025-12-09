use std::sync::Arc;

use actix_cors::Cors;
use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpServer, middleware, web};
use actix_web_flash_messages::{FlashMessagesFramework, storage::CookieMessageStore};
use pushkind_common::db::establish_connection_pool;
use pushkind_common::middleware::RedirectUnauthorized;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{logout, not_assigned};
use pushkind_common::zmq::{ZmqSender, ZmqSenderOptions};
use tera::Tera;

use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::routes::api::api_v1_clients;
use crate::routes::client::{attachment_client, comment_client, save_client, show_client};
use crate::routes::important_fields::{save_important_fields, show_important_fields};
use crate::routes::main::{add_client, clients_upload, show_index};
use crate::routes::managers::{add_manager, assign_manager, managers, managers_modal};

pub mod domain;
pub mod dto;
pub mod forms;
pub mod models;
pub mod repository;
pub mod routes;
pub mod schema;
pub mod services;

pub const SERVICE_ACCESS_ROLE: &str = "crm";
pub const SERVICE_ADMIN_ROLE: &str = "crm_admin";

/// Builds and runs the Actix-Web HTTP server using the provided configuration.
pub async fn run(server_config: ServerConfig) -> std::io::Result<()> {
    let common_config = CommonServerConfig {
        auth_service_url: server_config.auth_service_url.to_string(),
        secret: server_config.secret.clone(),
    };

    // Start a background ZeroMQ publisher used for outbound email notifications.
    let zmq_sender = ZmqSender::start(ZmqSenderOptions::pub_default(
        &server_config.zmq_emailer_pub,
    ))
    .map_err(|e| std::io::Error::other(format!("Failed to start ZMQ sender: {e}")))?;

    let zmq_sender = Arc::new(zmq_sender);

    // Establish Diesel connection pool for the SQLite database.
    let pool = establish_connection_pool(&server_config.database_url).map_err(|e| {
        std::io::Error::other(format!("Failed to establish database connection: {e}"))
    })?;

    let repo = DieselRepository::new(pool);

    // Keys and stores for identity, sessions, and flash messages.
    let secret_key = Key::from(server_config.secret.as_bytes());

    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    let tera = Tera::new(&server_config.templates_dir)
        .map_err(|e| std::io::Error::other(format!("Template parsing error(s): {e}")))?;

    let bind_address = (server_config.address.clone(), server_config.port);

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .wrap(message_framework.clone())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false) // set to true in prod
                    .cookie_domain(Some(format!(".{}", server_config.domain)))
                    .build(),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(Files::new("/assets", "./assets"))
            .service(not_assigned)
            .service(web::scope("/api").service(api_v1_clients))
            .service(
                web::scope("")
                    .wrap(RedirectUnauthorized)
                    .service(show_index)
                    .service(add_client)
                    .service(clients_upload)
                    .service(show_client)
                    .service(save_client)
                    .service(comment_client)
                    .service(attachment_client)
                    .service(show_important_fields)
                    .service(save_important_fields)
                    .service(managers)
                    .service(add_manager)
                    .service(managers_modal)
                    .service(assign_manager)
                    .service(logout),
            )
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config.clone()))
            .app_data(web::Data::new(zmq_sender.clone()))
            .app_data(web::Data::new(server_config.clone()))
    })
    .bind(bind_address)?
    .run()
    .await
}
