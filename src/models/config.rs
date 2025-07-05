#[derive(Clone)]
pub struct ServerConfig {
    pub secret: String,
    pub auth_service_url: String,
}
