use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use serde::Deserialize;
use tera::Tera;

use crate::forms::main::{AddClientForm, UploadClientsForm};
use crate::repository::DieselRepository;
use crate::services::{ServiceError, main as main_service};

#[derive(Deserialize)]
struct IndexQueryParams {
    q: Option<String>,
    page: Option<usize>,
}
#[get("/")]
pub async fn show_index(
    params: web::Query<IndexQueryParams>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let query = main_service::IndexQuery {
        search: params.q.clone(),
        page: params.page,
    };

    match main_service::load_index_page(repo.get_ref(), &user, query) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "index",
                &server_config.auth_service_url,
            );
            context.insert("clients", &data.clients);
            if let Some(search_query) = data.search_query.as_ref() {
                context.insert("search_query", search_query);
            }
            render_template(&tera, "main/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to list clients: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/client/add")]
pub async fn add_client(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AddClientForm>,
) -> impl Responder {
    match main_service::add_client(repo.get_ref(), &user, form) {
        Ok(outcome) => {
            FlashMessage::success(outcome.message).send();
            redirect(&outcome.redirect_to)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to add a client: {err}");
            FlashMessage::error("Ошибка при добавлении клиента").send();
            redirect("/")
        }
    }
}

#[post("/clients/upload")]
pub async fn clients_upload(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    MultipartForm(mut form): MultipartForm<UploadClientsForm>,
) -> impl Responder {
    match main_service::upload_clients(repo.get_ref(), &user, &mut form) {
        Ok(outcome) => {
            FlashMessage::success(outcome.message).send();
            redirect(&outcome.redirect_to)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to add clients: {err}");
            FlashMessage::error("Ошибка при добавлении клиентов").send();
            redirect("/")
        }
    }
}
