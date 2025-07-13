use actix_identity::Identity;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use log::error;
use serde::Deserialize;
use tera::Context;

use crate::db::DbPool;
use crate::domain::client::{Client, NewClient};
use crate::domain::manager::NewManager;
use crate::forms::main::AddClientForm;
use crate::models::auth::AuthenticatedUser;
use crate::models::config::ServerConfig;
use crate::pagination::Paginated;
use crate::repository::client::DieselClientRepository;
use crate::repository::manager::DieselManagerRepository;
use crate::repository::{ClientRepository, ManagerRepository};
use crate::routes::{alert_level_to_str, check_role, ensure_role, redirect, render_template};

#[derive(Deserialize)]
struct IndexQueryParams {
    page: Option<usize>,
}

#[get("/")]
pub async fn index(
    params: web::Query<IndexQueryParams>,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    };

    let page = params.page.unwrap_or(1);
    let client_repo = DieselClientRepository::new(&pool);
    let mut context = Context::new();

    if check_role("crm_admin", &user.roles) {
        let clients = match client_repo.list(user.hub_id, page) {
            Ok(clients) => clients,
            Err(e) => {
                error!("Failed to list clients: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };
        context.insert("clients", &clients);
    } else if check_role("crm_manager", &user.roles) {
        let manager_repo = DieselManagerRepository::new(&pool);
        let manager = match manager_repo.create_or_update(&NewManager {
            hub_id: user.hub_id,
            name: &user.name,
            email: &user.email,
        }) {
            Ok(manager) => manager,
            Err(e) => {
                error!("Failed to update manager: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };
        let clients = match client_repo.list_by_manager(&manager.email, manager.hub_id, page) {
            Ok(clients) => clients,
            Err(e) => {
                error!("Failed to list clients: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };
        context.insert("clients", &clients);
    } else {
        let clients: Paginated<Client> = Paginated::new(vec![], 0, 0);
        context.insert("clients", &clients);
    }

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();

    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "index");
    context.insert("home_url", &server_config.auth_service_url);

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
    pool: web::Data<DbPool>,
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

    let client_repo = DieselClientRepository::new(&pool);

    let clients = match client_repo.search(user.hub_id, query, page) {
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

#[post("/client/add")]
pub async fn add_client(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddClientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm_admin", Some("/na")) {
        return response;
    };

    let new_client: NewClient = (&form).into();

    let repo = DieselClientRepository::new(&pool);
    match repo.create(&[new_client]) {
        Ok(_) => {
            FlashMessage::success("Клиент добавлен.".to_string()).send();
        }
        Err(err) => {
            error!("Failed to add a client: {err}");
            FlashMessage::error(format!("Ошибка при добавлении клиента: {err}")).send();
        }
    }
    redirect("/")
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
