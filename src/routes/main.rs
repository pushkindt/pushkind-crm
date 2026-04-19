//! Routes for the main dashboard and uploads.

use actix_multipart::form::MultipartForm;
use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::dto::mutation::{ApiMutationErrorDto, ApiMutationSuccessDto};
use pushkind_common::routes::{check_role, redirect};

use crate::SERVICE_ACCESS_ROLE;
use crate::forms::main::{AddClientForm, AddClientPayload, UploadClientsForm};
use crate::frontend::{FrontendAssetError, open_frontend_html};
use crate::repository::DieselRepository;
use crate::routes::{MutationResource, mutation_error_response};
use crate::services::main as main_service;

#[get("/")]
/// Display the dashboard listing clients with optional search/pagination.
pub async fn show_index(request: HttpRequest, user: AuthenticatedUser) -> impl Responder {
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return redirect("/na?required_role=crm");
    }

    match open_frontend_html("assets/dist/app/index.html").await {
        Ok(file) => file.into_response(&request),
        Err(FrontendAssetError::Read(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            HttpResponse::ServiceUnavailable()
                .body("CRM frontend assets are not built yet. Run `cd frontend && npm run build`.")
        }
        Err(error) => {
            log::error!("Failed to open CRM index document: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/client/add")]
/// Handle client creation requests submitted from the dashboard.
pub async fn add_client(
    web::Form(form): web::Form<AddClientForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let payload = match AddClientPayload::try_from(form) {
        Ok(payload) => payload,
        Err(error) => {
            log::error!("Invalid add-client data: {error}");
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&error));
        }
    };

    match main_service::add_client(payload, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Created().json(ApiMutationSuccessDto {
            message: "Клиент добавлен.".to_string(),
            redirect_to: None,
        }),
        Err(err) => {
            log::error!("Failed to add a client: {err}");
            mutation_error_response(MutationResource::Client, &err)
        }
    }
}

#[post("/clients/upload")]
/// Accept a multipart upload of clients and delegate bulk import logic.
pub async fn clients_upload(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    MultipartForm(mut form): MultipartForm<UploadClientsForm>,
) -> impl Responder {
    match main_service::upload_clients(&mut form, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Клиенты добавлены.".to_string(),
            redirect_to: None,
        }),
        Err(err) => {
            log::error!("Failed to add clients: {err}");
            mutation_error_response(MutationResource::ClientImport, &err)
        }
    }
}
