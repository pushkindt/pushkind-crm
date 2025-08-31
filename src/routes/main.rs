use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::pagination::Paginated;
use pushkind_common::routes::{base_context, render_template};
use pushkind_common::routes::{check_role, ensure_role, redirect};
use serde::Deserialize;
use tera::Tera;
use validator::Validate;

use crate::domain::client::NewClient;
use crate::forms::main::{AddClientForm, UploadClientsForm};
use crate::repository::{
    ClientListQuery, ClientReader, ClientWriter, DieselRepository, ManagerWriter,
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
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    }

    let page = params.page.unwrap_or(1);
    let q = params.q.as_deref().unwrap_or("").trim();

    let mut context = base_context(
        &flash_messages,
        &user,
        "index",
        &server_config.auth_service_url,
    );

    let clients_result = if !q.is_empty() {
        context.insert("search_query", q);
        repo.search_clients(
            ClientListQuery::new(user.hub_id)
                .search(q)
                .paginate(page, DEFAULT_ITEMS_PER_PAGE),
        )
    } else if check_role("crm_admin", &user.roles) {
        repo.list_clients(ClientListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE))
    } else if check_role("crm_manager", &user.roles) {
        match repo.create_or_update_manager(&(&user).into()) {
            Ok(manager) => repo.list_clients(
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

    context.insert("clients", &clients);
    if !q.is_empty() {
        context.insert("search_query", q); // optional: show search term in UI
    }

    render_template(&tera, "main/index.html", &context)
}

#[post("/client/add")]
pub async fn add_client(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
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

    let new_client: NewClient = form.to_new_client(user.hub_id);

    match repo.create_clients(&[new_client]) {
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

#[post("/clients/upload")]
pub async fn clients_upload(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    MultipartForm(mut form): MultipartForm<UploadClientsForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm_admin", Some("/na")) {
        return response;
    };

    let clients = match form.parse(user.hub_id) {
        Ok(clients) => clients,
        Err(err) => {
            log::error!("Failed to parse clients: {err}");
            FlashMessage::error("Ошибка при парсинге клиентов").send();
            return redirect("/");
        }
    };

    match repo.create_clients(&clients) {
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
