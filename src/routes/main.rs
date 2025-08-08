use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::pagination::Paginated;
use pushkind_common::routes::{alert_level_to_str, check_role, ensure_role, redirect};
use serde::Deserialize;
use tera::Context;
use validator::Validate;

use crate::domain::client::NewClient;
use crate::forms::main::{AddClientForm, UploadClientsForm};
use crate::repository::client::DieselClientRepository;
use crate::repository::manager::DieselManagerRepository;
use crate::repository::{ClientListQuery, ClientReader, ClientWriter, ManagerWriter};
use crate::routes::render_template;

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
    server_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    }

    let page = params.page.unwrap_or(1);
    let q = params.q.as_deref().unwrap_or("").trim();
    let client_repo = DieselClientRepository::new(&pool);
    let mut context = Context::new();

    let clients_result = if !q.is_empty() {
        context.insert("search_query", q);
        client_repo.search(
            ClientListQuery::new(user.hub_id)
                .search(q)
                .paginate(page, DEFAULT_ITEMS_PER_PAGE),
        )
    } else if check_role("crm_admin", &user.roles) {
        client_repo.list(ClientListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE))
    } else if check_role("crm_manager", &user.roles) {
        let manager_repo = DieselManagerRepository::new(&pool);
        match manager_repo.create_or_update(&(&user).into()) {
            Ok(manager) => client_repo.list(
                ClientListQuery::new(user.hub_id)
                    .manager_email(&manager.email)
                    .paginate(page, DEFAULT_ITEMS_PER_PAGE),
            ),
            Err(e) => {
                log::error!("Failed to update manager: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        }
    } else {
        Ok((0, vec![]))
    };

    let clients = match clients_result {
        Ok((total, clients)) => {
            Paginated::new(clients, page, total.div_ceil(DEFAULT_ITEMS_PER_PAGE))
        }
        Err(e) => {
            log::error!("Failed to list clients: {e}");
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

    if let Err(e) = form.validate() {
        log::error!("Failed to validate form: {e}");
        FlashMessage::error("Ошибка валидации формы").send();
        return redirect("/");
    }

    if form.hub_id != user.hub_id {
        log::warn!(
            "Attempt to add client to a different hub: form_hub_id={}, user_hub_id={}",
            form.hub_id,
            user.hub_id
        );
        FlashMessage::error("Ошибка при добавлении клиента".to_string()).send();
        return redirect("/");
    }

    let new_client: NewClient = form.into();

    let repo = DieselClientRepository::new(&pool);
    match repo.create(&[new_client]) {
        Ok(_) => {
            FlashMessage::success("Клиент добавлен.".to_string()).send();
        }
        Err(err) => {
            log::error!("Failed to add a client: {err}");
            FlashMessage::error("Ошибка при добавлении клиента").send();
        }
    }
    redirect("/")
}

#[get("/na")]
pub async fn not_assigned(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
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

#[post("/clients/upload")]
pub async fn clients_upload(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    MultipartForm(mut form): MultipartForm<UploadClientsForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm_admin", Some("/na")) {
        return response;
    };

    let client_repo = DieselClientRepository::new(&pool);

    let clients = match form.parse(user.hub_id) {
        Ok(clients) => clients,
        Err(err) => {
            log::error!("Failed to parse clients: {err}");
            FlashMessage::error("Ошибка при парсинге клиентов").send();
            return redirect("/");
        }
    };

    match client_repo.create(&clients) {
        Ok(_) => {
            FlashMessage::success("Клиенты добавлены.".to_string()).send();
        }
        Err(err) => {
            log::error!("Failed to add clients: {err}");
            FlashMessage::error("Ошибка при добавлении клиентов").send();
        }
    }

    redirect("/")
}
