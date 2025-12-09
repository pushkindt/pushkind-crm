//! Actix routes for client CRUD interactions.

use std::sync::Arc;

use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::zmq::ZmqSender;
use tera::Tera;

use crate::forms::client::{AddAttachmentForm, AddCommentForm, SaveClientForm};
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, client as client_service};

#[get("/client/{client_id}")]
pub async fn show_client(
    client_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    common_config: web::Data<CommonServerConfig>,
    server_config: web::Data<ServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let client_id = client_id.into_inner();
    let repo = repo.get_ref();

    match client_service::load_client_details(repo, &user, client_id) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "index",
                &common_config.auth_service_url,
            );
            context.insert("client", &data.client);
            context.insert("managers", &data.managers);
            context.insert("events", &data.events_with_managers);
            context.insert("documents", &data.documents);
            context.insert("available_fields", &data.available_fields);
            context.insert("important_fields", &data.important_fields);
            context.insert("other_fields", &data.other_fields);
            context.insert("todo_service_url", &server_config.todo_service_url);

            render_template(&tera, "client/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Этот клиент для вас не доступен").send();
            redirect("/")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Клиент не найден.").send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to load client {client_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/client/save")]
pub async fn save_client(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Bytes,
) -> impl Responder {
    let repo = repo.get_ref();

    let form: SaveClientForm = match serde_html_form::from_bytes(&form) {
        Ok(form) => form,
        Err(err) => {
            log::error!("Error parsing form: {err}");
            FlashMessage::error("Ошибка при обработке формы.").send();
            return redirect("/clients");
        }
    };

    let client_id = form.id;

    match client_service::save_client(repo, &user, form) {
        Ok(result) => {
            FlashMessage::success("Клиент обновлен.".to_string()).send();
            redirect(&format!("/client/{}", result.client_id))
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Этот клиент для вас не доступен").send();
            redirect("/")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Клиент не найден.").send();
            redirect("/")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect(&format!("/client/{}", client_id))
        }
        Err(err) => {
            log::error!("Failed to update client {client_id}: {err}");
            FlashMessage::error("Ошибка при обновлении клиента").send();
            redirect(&format!("/client/{}", client_id))
        }
    }
}

#[post("/client/comment")]
pub async fn comment_client(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
    web::Form(form): web::Form<AddCommentForm>,
) -> impl Responder {
    let repo = repo.get_ref();
    let client_id = form.id;
    let sender = zmq_sender.get_ref().as_ref();

    match client_service::add_comment(repo, &user, sender, form).await {
        Ok(result) => {
            FlashMessage::success("Событие добавлено.".to_string()).send();
            redirect(&format!("/client/{}", result.client_id))
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Этот клиент для вас не доступен").send();
            redirect("/")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Клиент не найден.").send();
            redirect("/")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect(&format!("/client/{}", client_id))
        }
        Err(ServiceError::Internal) => {
            FlashMessage::error("Ошибка при добавлении сообщения в очередь.").send();
            redirect(&format!("/client/{}", client_id))
        }
        Err(err) => {
            log::error!("Failed to add comment for client {client_id}: {err}");
            FlashMessage::error("Ошибка при добавлении события").send();
            redirect(&format!("/client/{}", client_id))
        }
    }
}

#[post("/client/attachment")]
pub async fn attachment_client(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AddAttachmentForm>,
) -> impl Responder {
    let repo = repo.get_ref();
    let client_id = form.id;

    match client_service::add_attachment(repo, &user, form) {
        Ok(result) => {
            FlashMessage::success("Событие добавлено.".to_string()).send();
            redirect(&format!("/client/{}", result.client_id))
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Этот клиент для вас не доступен").send();
            redirect("/")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Клиент не найден.").send();
            redirect("/")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect(&format!("/client/{}", client_id))
        }
        Err(err) => {
            log::error!("Failed to add attachment for client {client_id}: {err}");
            FlashMessage::error("Ошибка при добавлении события").send();
            redirect(&format!("/client/{}", client_id))
        }
    }
}
