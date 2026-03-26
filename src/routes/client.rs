//! Actix routes for client CRUD interactions.

use std::sync::Arc;

use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::redirect;
use pushkind_common::zmq::ZmqSender;

use crate::dto::api::{ApiMutationErrorDto, ApiMutationSuccessDto};
use crate::forms::client::{
    AddAttachmentForm, AddAttachmentPayload, AddCommentForm, AddCommentPayload, SaveClientForm,
    SaveClientPayload,
};
use crate::frontend::{FrontendAssetError, open_frontend_html};
use crate::repository::DieselRepository;
use crate::routes::{MutationResource, mutation_error_response};
use crate::services::{ServiceError, client as client_service};

#[get("/client/{client_id}")]
/// Render the detail page for a single client, including events and attachments.
pub async fn show_client(
    request: HttpRequest,
    client_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let client_id = client_id.into_inner();
    let repo = repo.get_ref();

    match client_service::verify_client_page_access(client_id, &user, repo) {
        Ok(_) => match open_frontend_html("assets/dist/app/client.html").await {
            Ok(file) => file.into_response(&request),
            Err(FrontendAssetError::Read(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                HttpResponse::ServiceUnavailable().body(
                    "CRM frontend assets are not built yet. Run `cd frontend && npm run build`.",
                )
            }
            Err(error) => {
                log::error!("Failed to open CRM client document: {error}");
                HttpResponse::InternalServerError().finish()
            }
        },
        Err(ServiceError::Unauthorized) => redirect("/"),
        Err(ServiceError::NotFound) => redirect("/"),
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
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::default());
        }
    };

    let client_id = client_id.into_inner();
    let payload = match SaveClientPayload::try_from(form) {
        Ok(payload) => payload,
        Err(error) => {
            log::error!("Invalid save-client data for client {client_id}: {error}");
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&error));
        }
    };

    match client_service::save_client(client_id, payload, &user, repo) {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Клиент обновлен.".to_string(),
            redirect_to: None,
        }),
        Err(err) => {
            log::error!("Failed to update client {client_id}: {err}");
            mutation_error_response(MutationResource::Client, &err)
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
    let payload = match AddCommentPayload::try_from(form) {
        Ok(payload) => payload,
        Err(error) => {
            log::error!("Invalid comment data for client {client_id}: {error}");
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&error));
        }
    };

    match client_service::add_comment(client_id, payload, &user, repo, sender).await {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Событие добавлено.".to_string(),
            redirect_to: None,
        }),
        Err(err) => {
            log::error!("Failed to add comment for client {client_id}: {err}");
            mutation_error_response(MutationResource::ClientComment, &err)
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
    let payload = match AddAttachmentPayload::try_from(form) {
        Ok(payload) => payload,
        Err(error) => {
            log::error!("Invalid attachment data for client {client_id}: {error}");
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&error));
        }
    };

    match client_service::add_attachment(client_id, payload, &user, repo) {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Событие добавлено.".to_string(),
            redirect_to: None,
        }),
        Err(err) => {
            log::error!("Failed to add attachment for client {client_id}: {err}");
            mutation_error_response(MutationResource::ClientComment, &err)
        }
    }
}
