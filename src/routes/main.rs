use actix_identity::Identity;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use log::error;
use serde::Deserialize;
use tera::Context;

use crate::db::DbPool;
use crate::domain::client::NewClient;
use crate::domain::manager::NewManager;
use crate::forms::main::AddClientForm;
use crate::models::auth::AuthenticatedUser;
use crate::models::config::ServerConfig;
use crate::pagination::Paginated;
use crate::repository::client::DieselClientRepository;
use crate::repository::manager::DieselManagerRepository;
use crate::repository::{
    ClientListQuery, ClientReader, ClientSearchQuery, ClientWriter, ManagerWriter,
};
use crate::routes::{
    DEFAULT_ITEMS_PER_PAGE, alert_level_to_str, check_role, ensure_role, redirect, render_template,
};

#[derive(Deserialize)]
struct IndexQueryParams {
    q: Option<String>,
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
    }

    let page = params.page.unwrap_or(1);
    let q = params.q.as_deref().unwrap_or("").trim();
    let client_repo = DieselClientRepository::new(&pool);
    let mut context = Context::new();

    let clients_result = if !q.is_empty() {
        client_repo
            .search(ClientSearchQuery::new(user.hub_id, q).paginate(page, DEFAULT_ITEMS_PER_PAGE))
    } else if check_role("crm_admin", &user.roles) {
        client_repo.list(ClientListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE))
    } else if check_role("crm_manager", &user.roles) {
        let manager_repo = DieselManagerRepository::new(&pool);
        match manager_repo.create_or_update(&NewManager {
            hub_id: user.hub_id,
            name: &user.name,
            email: &user.email,
        }) {
            Ok(manager) => client_repo.list(
                ClientListQuery::new(user.hub_id)
                    .manager_email(&manager.email)
                    .paginate(page, DEFAULT_ITEMS_PER_PAGE),
            ),
            Err(e) => {
                error!("Failed to update manager: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        }
    } else {
        Ok((0, vec![]))
    };

    let clients = match clients_result {
        Ok((total, clients)) => Paginated::new(
            clients,
            page,
            ((total + DEFAULT_ITEMS_PER_PAGE - 1) / DEFAULT_ITEMS_PER_PAGE) as usize,
        ),
        Err(e) => {
            error!("Failed to list clients: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();

    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "index");
    context.insert("home_url", &server_config.auth_service_url);
    context.insert("clients", &clients);
    if !q.is_empty() {
        context.insert("search_query", q); // optional: show search term in UI
    }

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
