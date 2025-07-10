use actix_identity::Identity;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::IncomingFlashMessages;
use log::error;
use serde::Deserialize;
use tera::Context;

use crate::domain::manager::NewManager;
use crate::models::auth::AuthenticatedUser;
use crate::models::config::ServerConfig;
use crate::repository::test::{TestClientRepository, TestManagerRepository};
use crate::repository::{ClientRepository, ManagerRepository};
use crate::routes::{alert_level_to_str, check_role, ensure_role, redirect, render_template};

#[get("/settings")]
pub async fn settings(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    };

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "settings");
    context.insert("home_url", &server_config.auth_service_url);

    render_template("settings/index.html", &context)
}
