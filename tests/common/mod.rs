#![allow(dead_code)]

//! Helpers for integration tests.

use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use actix_cors::Cors;
use actix_identity::{Identity, IdentityMiddleware};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::rt::time::sleep;
use actix_web::{
    App, HttpMessage, HttpRequest, HttpResponse, HttpServer, Responder, middleware, post, web,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use pushkind_common::db::{DbPool, establish_connection_pool};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::middleware::RedirectUnauthorized;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::logout;
use pushkind_common::zmq::{ZmqSender, ZmqSenderOptions};
use reqwest::{Client, StatusCode, redirect::Policy};
use tempfile::NamedTempFile;

use pushkind_crm::models::config::AppConfig;
use pushkind_crm::repository::DieselRepository;
use pushkind_crm::routes::api::{
    api_v1_client_details, api_v1_client_directory, api_v1_clients, api_v1_iam,
    api_v1_important_fields, api_v1_manager_modal, api_v1_managers, api_v1_no_access,
};
use pushkind_crm::routes::aux::not_assigned;
use pushkind_crm::routes::client::{attachment_client, comment_client, save_client, show_client};
use pushkind_crm::routes::main::{add_client, clients_upload, show_index};
use pushkind_crm::routes::managers::{add_manager, assign_manager, managers};
use pushkind_crm::routes::settings::{cleanup_clients, save_important_fields, show_settings};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!(); // assumes migrations/ exists
pub const HUB_ID: i32 = 7;

/// Temporary database used in integration tests.
pub struct TestDb {
    _tempfile: NamedTempFile,
    pool: DbPool,
}

pub struct TestApp {
    test_db: TestDb,
    address: String,
}

impl TestDb {
    pub fn new() -> Self {
        let tempfile = NamedTempFile::new().expect("Failed to create temp file");
        let pool = establish_connection_pool(tempfile.path().to_str().unwrap())
            .expect("Failed to establish SQLite connection.");
        let mut conn = pool
            .get()
            .expect("Failed to get SQLite connection from pool.");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Migrations failed");
        TestDb {
            _tempfile: tempfile,
            pool,
        }
    }

    pub fn pool(&self) -> DbPool {
        self.pool.clone()
    }

    pub fn get_db_path(&self) -> String {
        self._tempfile.path().to_str().unwrap().to_string()
    }
}

impl TestApp {
    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn db_pool(&self) -> DbPool {
        self.test_db.pool()
    }

    pub fn repo(&self) -> DieselRepository {
        DieselRepository::new(self.db_pool())
    }
}

#[derive(serde::Deserialize)]
struct LoginRequest {
    hub_id: i32,
    email: String,
    name: String,
    roles: Vec<String>,
}

#[post("/test/login")]
async fn test_login(
    request: HttpRequest,
    payload: web::Json<LoginRequest>,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    let mut user = AuthenticatedUser {
        sub: payload.email.clone(),
        email: payload.email.clone(),
        hub_id: payload.hub_id,
        name: payload.name.clone(),
        roles: payload.roles.clone(),
        exp: 0,
    };
    user.set_expiration(7);

    let token = user
        .to_jwt(&common_config.secret)
        .expect("JWT generation should succeed for test users.");
    Identity::login(&request.extensions(), token).expect("Test login should persist identity.");

    HttpResponse::Ok().finish()
}

async fn wait_until_server_is_ready(address: &str) {
    let client = Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_millis(100))
        .build()
        .expect("Failed to create the test HTTP client.");
    let url = format!("{address}/");

    for _ in 0..20 {
        match client.get(&url).send().await {
            Ok(response)
                if response.status() == StatusCode::SEE_OTHER
                    || response.status() == StatusCode::OK =>
            {
                return;
            }
            Ok(_) | Err(_) => sleep(Duration::from_millis(25)).await,
        }
    }

    panic!("Test server did not become ready at {url}");
}

pub async fn spawn_app() -> TestApp {
    let test_db = TestDb::new();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind a random local port.");
    let port = listener
        .local_addr()
        .expect("Failed to read the local socket address.")
        .port();

    let app_config = AppConfig {
        domain: "localhost".to_string(),
        database_url: test_db.get_db_path(),
        zmq_emailer_pub: "tcp://127.0.0.1:35557".to_string(),
        zmq_emailer_sub: "tcp://127.0.0.1:35558".to_string(),
        zmq_sms_pub: "tcp://127.0.0.1:35561".to_string(),
        zmq_clients_sub: "tcp://127.0.0.1:35566".to_string(),
        zmq_replier_sub: "tcp://127.0.0.1:35560".to_string(),
        zmq_tasks_sub: "tcp://127.0.0.1:35564".to_string(),
        sms_sender: "crm-test".to_string(),
        secret: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        auth_service_url: "https://users.pushkind.test/auth/signin".to_string(),
        todo_service_url: "https://todo.pushkind.test".to_string(),
        files_service_url: "https://files.pushkind.test".to_string(),
    };
    let common_config = CommonServerConfig {
        auth_service_url: app_config.auth_service_url.clone(),
        secret: app_config.secret.clone(),
    };
    let secret_key = Key::from(app_config.secret.as_bytes());
    let repo = DieselRepository::new(test_db.pool());
    let zmq_sender = Arc::new(
        ZmqSender::start(ZmqSenderOptions::pub_default("tcp://127.0.0.1:35559"))
            .expect("Failed to start test ZMQ sender."),
    );

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false)
                    .build(),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(actix_files::Files::new("/assets", "./assets"))
            .service(test_login)
            .service(not_assigned)
            .service(
                web::scope("/api")
                    .service(api_v1_iam)
                    .service(api_v1_clients)
                    .service(api_v1_client_directory)
                    .service(api_v1_client_details)
                    .service(api_v1_managers)
                    .service(api_v1_manager_modal)
                    .service(api_v1_no_access)
                    .service(api_v1_important_fields),
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
    .listen(listener)
    .expect("Failed to listen with the test server.")
    .run();

    actix_web::rt::spawn(server);
    let address = format!("http://127.0.0.1:{port}");

    wait_until_server_is_ready(&address).await;

    TestApp { test_db, address }
}

pub fn build_reqwest_client() -> Client {
    Client::builder()
        .cookie_store(true)
        .build()
        .expect("Can't create a request client")
}

pub fn build_no_redirect_client() -> Client {
    Client::builder()
        .cookie_store(true)
        .redirect(Policy::none())
        .build()
        .expect("Can't create a request client")
}

pub async fn login_as(
    client: &Client,
    address: &str,
    email: &str,
    name: &str,
    hub_id: i32,
    roles: &[&str],
) {
    let response = client
        .post(format!("{address}/test/login"))
        .json(&serde_json::json!({
            "hub_id": hub_id,
            "email": email,
            "name": name,
            "roles": roles,
        }))
        .send()
        .await
        .expect("Failed to submit test login.");

    assert_eq!(response.status(), StatusCode::OK);
}
