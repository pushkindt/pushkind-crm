//! CRM application module wiring domain, repository, and HTTP services.

#[cfg(feature = "server")]
use std::sync::Arc;

#[cfg(feature = "server")]
use crate::models::config::{AppConfig, Settings};
#[cfg(feature = "server")]
use crate::repository::DieselRepository;
#[cfg(feature = "server")]
use crate::routes::api::{
    api_v1_client_details, api_v1_clients, api_v1_dashboard, api_v1_iam, api_v1_manager_modal,
    api_v1_managers, api_v1_no_access, api_v1_settings,
};
#[cfg(feature = "server")]
use crate::routes::aux::not_assigned;
#[cfg(feature = "server")]
use crate::routes::client::{attachment_client, comment_client, save_client, show_client};
#[cfg(feature = "server")]
use crate::routes::main::{add_client, clients_upload, show_index};
#[cfg(feature = "server")]
use crate::routes::managers::{add_manager, assign_manager, managers};
#[cfg(feature = "server")]
use crate::routes::rate_limit::{StoreOtpIpRateLimiter, TRUST_FORWARDED_HEADERS};
#[cfg(feature = "server")]
use crate::routes::settings::{cleanup_clients, save_important_fields, show_settings};
#[cfg(feature = "server")]
use crate::routes::store::{
    get_store_session, logout_store_session, request_store_auth_otp, verify_store_auth_otp,
};
#[cfg(feature = "server")]
use actix_cors::Cors;
#[cfg(feature = "server")]
use actix_files::Files;
#[cfg(feature = "server")]
use actix_identity::IdentityMiddleware;
#[cfg(feature = "server")]
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
#[cfg(feature = "server")]
use actix_web::cookie::Key;
#[cfg(feature = "server")]
use actix_web::{App, HttpServer, dev::Server, middleware, web};
#[cfg(feature = "server")]
use pushkind_common::db::establish_connection_pool;
#[cfg(feature = "server")]
use pushkind_common::middleware::RedirectUnauthorized;
#[cfg(feature = "server")]
use pushkind_common::models::config::CommonServerConfig;
#[cfg(feature = "server")]
use pushkind_common::routes::logout;
#[cfg(feature = "server")]
use pushkind_common::zmq::{ZmqSender, ZmqSenderOptions};

#[cfg(feature = "data")]
pub mod domain;
#[cfg(feature = "server")]
pub mod dto;
mod error_conversions;
#[cfg(feature = "server")]
pub mod forms;
#[cfg(feature = "server")]
mod frontend;
#[cfg(feature = "data")]
pub mod models;
#[cfg(feature = "server")]
pub mod repository;
#[cfg(feature = "server")]
pub mod routes;
#[cfg(feature = "data")]
pub mod schema;
#[cfg(feature = "server")]
pub mod services;

pub const SERVICE_ACCESS_ROLE: &str = "crm";
pub const SERVICE_ADMIN_ROLE: &str = "crm_admin";
pub const SERVICE_MANAGER_ROLE: &str = "crm_manager";

/// Builds and runs the Actix-Web HTTP server using the provided configuration.
#[cfg(feature = "server")]
pub async fn run(settings: Settings) -> std::io::Result<()> {
    let listener =
        std::net::TcpListener::bind((settings.server.address.clone(), settings.server.port))?;

    build_server(listener, settings.app)?.await
}

#[cfg(feature = "server")]
pub fn build_server(
    listener: std::net::TcpListener,
    app_config: AppConfig,
) -> std::io::Result<Server> {
    let common_config = CommonServerConfig {
        auth_service_url: app_config.auth_service_url.to_string(),
        secret: app_config.secret.clone(),
    };

    // Start a background ZeroMQ publisher used for outbound email notifications.
    let zmq_sender =
        ZmqSender::start(ZmqSenderOptions::pub_default(&app_config.zmq_emailer_pub))
            .map_err(|e| std::io::Error::other(format!("Failed to start ZMQ sender: {e}")))?;

    let zmq_sender = Arc::new(zmq_sender);
    let sms_sender = ZmqSender::start(ZmqSenderOptions::pub_default(&app_config.zmq_sms_pub))
        .map_err(|e| std::io::Error::other(format!("Failed to start ZMQ SMS sender: {e}")))?;

    // Establish Diesel connection pool for the SQLite database.
    let pool = establish_connection_pool(&app_config.database_url).map_err(|e| {
        std::io::Error::other(format!("Failed to establish database connection: {e}"))
    })?;

    let repo = DieselRepository::new(pool);

    // Keys and stores for identity and sessions.
    let secret_key = Key::from(app_config.secret.as_bytes());
    let store_otp_rate_limiter = web::Data::new(StoreOtpIpRateLimiter::new());
    if !TRUST_FORWARDED_HEADERS {
        log::warn!(
            "CRM store OTP rate limiter uses peer_addr() for client IP. \
If this service runs behind a trusted reverse proxy, set TRUST_FORWARDED_HEADERS=true in src/routes/rate_limit.rs."
        );
    }

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false) // set to true in prod
                    .cookie_domain(Some(format!(".{}", app_config.domain)))
                    .build(),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(Files::new("/assets", "./assets"))
            .service(not_assigned)
            .service(
                web::scope("/api/v1/store")
                    .app_data(store_otp_rate_limiter.clone())
                    .app_data(web::Data::new(sms_sender.clone()))
                    .service(request_store_auth_otp)
                    .service(verify_store_auth_otp)
                    .service(get_store_session)
                    .service(logout_store_session),
            )
            .service(
                web::scope("/api")
                    .service(api_v1_iam)
                    .service(api_v1_clients)
                    .service(api_v1_dashboard)
                    .service(api_v1_client_details)
                    .service(api_v1_managers)
                    .service(api_v1_manager_modal)
                    .service(api_v1_no_access)
                    .service(api_v1_settings),
            )
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
                    .service(show_settings)
                    .service(save_important_fields)
                    .service(cleanup_clients)
                    .service(managers)
                    .service(add_manager)
                    .service(assign_manager)
                    .service(logout),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(common_config.clone()))
            .app_data(web::Data::new(zmq_sender.clone()))
            .app_data(web::Data::new(app_config.clone()))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
