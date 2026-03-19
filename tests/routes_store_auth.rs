use actix_web::cookie::Cookie;
use actix_web::{App, http::StatusCode, test, web};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use pushkind_crm::domain::client::NewClient;
use pushkind_crm::domain::store_session::{STORE_SESSION_COOKIE_NAME, StoreSessionClaims};
use pushkind_crm::models::config::ServerConfig;
use pushkind_crm::repository::{ClientReader, ClientWriter, DieselRepository};
use pushkind_crm::routes::store::{get_store_session, logout_store_session};

mod common;

fn test_config() -> ServerConfig {
    ServerConfig {
        domain: "example.com".to_string(),
        address: "127.0.0.1".to_string(),
        port: 8080,
        database_url: "app.db".to_string(),
        zmq_emailer_pub: "tcp://127.0.0.1:5557".to_string(),
        zmq_emailer_sub: "tcp://127.0.0.1:5558".to_string(),
        zmq_sms_pub: "tcp://127.0.0.1:5561".to_string(),
        zmq_clients_sub: "tcp://127.0.0.1:5566".to_string(),
        zmq_replier_sub: "tcp://127.0.0.1:5560".to_string(),
        zmq_tasks_sub: "tcp://127.0.0.1:5564".to_string(),
        sms_sender: "cns.shared".to_string(),
        templates_dir: "templates/**/*".to_string(),
        secret: "secret".to_string(),
        auth_service_url: "".to_string(),
        todo_service_url: "".to_string(),
        files_service_url: "".to_string(),
    }
}

#[actix_web::test]
async fn get_store_session_returns_customer_for_valid_cookie() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    repo.create_or_replace_clients(&[NewClient::try_new(
        7,
        "Alice".to_string(),
        Some("alice@example.com".to_string()),
        Some("+15551234567".to_string()),
        None,
    )
    .unwrap()])
        .unwrap();
    let client = repo
        .get_client_by_phone(
            &pushkind_crm::domain::types::PhoneNumber::new("+15551234567").unwrap(),
            pushkind_crm::domain::types::HubId::new(7).unwrap(),
        )
        .unwrap()
        .unwrap();

    let claims = StoreSessionClaims {
        sub: client.public_id.unwrap().to_string(),
        hub_id: 7,
        name: "Alice".to_string(),
        phone: "+15551234567".to_string(),
        email: Some("alice@example.com".to_string()),
        exp: (Utc::now() + Duration::days(1)).timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("secret".as_bytes()),
    )
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(repo))
            .app_data(web::Data::new(test_config()))
            .service(web::scope("/api/v1/store").service(get_store_session)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/7/auth/session")
        .cookie(Cookie::new(STORE_SESSION_COOKIE_NAME, token))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn logout_store_session_clears_cookie() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(test_config()))
            .service(web::scope("/api/v1/store").service(logout_store_session)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/store/7/auth/logout")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let cookie = resp
        .response()
        .cookies()
        .find(|cookie| cookie.name() == STORE_SESSION_COOKIE_NAME)
        .expect("logout clears store cookie");
    assert_eq!(cookie.max_age().map(|age| age.whole_seconds()), Some(0));
}
