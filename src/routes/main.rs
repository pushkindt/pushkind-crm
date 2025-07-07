use actix_identity::Identity;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::IncomingFlashMessages;
use log::error;
use serde::Deserialize;
use tera::Context;

use crate::models::auth::AuthenticatedUser;
use crate::models::config::ServerConfig;
use crate::repository::ClientRepository;
use crate::repository::test::TestClientRepository;
use crate::routes::{alert_level_to_str, ensure_role, redirect, render_template};

#[derive(Deserialize)]
struct IndexQueryParams {
    page: Option<usize>,
}

#[get("/")]
pub async fn index(
    params: web::Query<IndexQueryParams>,
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    };

    let page = params.page.unwrap_or(1);

    let repo = TestClientRepository;

    let clients = match repo.list(user.hub_id, page) {
        Ok(clients) => clients,
        Err(e) => {
            error!("Failed to list clients: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "index");
    context.insert("home_url", &server_config.auth_service_url);
    context.insert("clients", &clients);

    render_template("main/index.html", &context)
}

#[derive(Deserialize)]
struct SearchQueryParams {
    q: Option<String>,
    page: Option<usize>,
}

#[get("/search")]
pub async fn search(
    params: web::Query<SearchQueryParams>,
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    };

    let page = params.page.unwrap_or(1);

    let query = match &params.q {
        Some(query) => query,
        None => "",
    };

    let repo = TestClientRepository;

    let clients = match repo.search(user.hub_id, query, page) {
        Ok(clients) => clients,
        Err(e) => {
            error!("Failed to list clients: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "index");
    context.insert("home_url", &server_config.auth_service_url);
    context.insert("clients", &clients);

    render_template("main/index.html", &context)
}

#[post("/logout")]
pub async fn logout(user: Identity) -> impl Responder {
    user.logout();
    redirect("/")
}

#[get("/na")]
pub async fn not_assigned(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "index");
    context.insert("home_url", &server_config.auth_service_url);

    render_template("main/not_assigned.html", &context)
}
