//! Configuration model loaded from external sources.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub app: AppConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize)]
/// Application configuration shared across handlers and background services.
pub struct AppConfig {
    pub domain: String,
    pub database_url: String,
    pub zmq_emailer_pub: String,
    pub zmq_emailer_sub: String,
    pub zmq_sms_pub: String,
    pub zmq_clients_sub: String,
    pub zmq_replier_sub: String,
    pub zmq_tasks_sub: String,
    pub sms_sender: String,
    pub secret: String,
    pub auth_service_url: String,
    pub todo_service_url: String,
    pub files_service_url: String,
}
