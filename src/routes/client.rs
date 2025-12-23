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
/// Render the detail page for a single client, including events and attachments.
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

    match client_service::load_client_details(client_id, &user, repo) {
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
            context.insert("files_service_url", &server_config.files_service_url);

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

#[post("/client/{client_id}/save")]
/// Persist updates to a client's profile submitted from the client form.
pub async fn save_client(
    client_id: web::Path<i32>,
    form: web::Bytes,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
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

    let client_id = client_id.into_inner();

    match client_service::save_client(client_id, form, &user, repo) {
        Ok(result) => {
            FlashMessage::success("Клиент обновлен.".to_string()).send();
            redirect(&format!("/client/{}", result.client_id.get()))
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
            FlashMessage::error(format!("Ошибка обработки формы: {message}")).send();
            redirect(&format!("/client/{}", client_id))
        }
        Err(err) => {
            log::error!("Failed to update client {client_id}: {err}");
            FlashMessage::error("Ошибка при обновлении клиента").send();
            redirect(&format!("/client/{}", client_id))
        }
    }
}

#[post("/client/{client_id}/comment")]
/// Queue a new comment event for the client via the ZMQ sender.
pub async fn comment_client(
    client_id: web::Path<i32>,
    web::Form(form): web::Form<AddCommentForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
) -> impl Responder {
    let repo = repo.get_ref();
    let client_id = client_id.into_inner();
    let sender = zmq_sender.get_ref().as_ref();

    match client_service::add_comment(client_id, form, &user, repo, sender).await {
        Ok(result) => {
            FlashMessage::success("Событие добавлено.".to_string()).send();
            redirect(&format!("/client/{}", result.client_id.get()))
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
            FlashMessage::error(format!("Ошибка обработки формы: {message}")).send();
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

#[post("/client/{client_id}/attachment")]
/// Upload and associate an attachment with the given client.
pub async fn attachment_client(
    client_id: web::Path<i32>,
    web::Form(form): web::Form<AddAttachmentForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let repo = repo.get_ref();
    let client_id = client_id.into_inner();

    match client_service::add_attachment(client_id, form, &user, repo) {
        Ok(result) => {
            FlashMessage::success("Событие добавлено.".to_string()).send();
            redirect(&format!("/client/{}", result.client_id.get()))
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
            FlashMessage::error(format!("Ошибка обработки формы: {message}")).send();
            redirect(&format!("/client/{}", client_id))
        }
        Err(err) => {
            log::error!("Failed to add attachment for client {client_id}: {err}");
            FlashMessage::error("Ошибка при добавлении события").send();
            redirect(&format!("/client/{}", client_id))
        }
    }
}
