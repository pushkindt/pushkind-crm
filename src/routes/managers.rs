//! Routes that manage manager assignments.

use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::dto::mutation::{ApiMutationErrorDto, ApiMutationSuccessDto};
use pushkind_common::routes::{check_role, redirect};

use crate::SERVICE_ADMIN_ROLE;
use crate::forms::managers::{
    AddManagerForm, AddManagerPayload, AssignManagerForm, AssignManagerPayload,
};
use crate::frontend::{FrontendAssetError, open_frontend_html};
use crate::repository::DieselRepository;
use crate::routes::{MutationResource, mutation_error_response};
use crate::services::managers as managers_service;

#[get("/managers")]
/// Render the managers list page, showing assignments and controls.
pub async fn managers(
    request: HttpRequest,
    user: AuthenticatedUser,
    _repo: web::Data<DieselRepository>,
) -> impl Responder {
    if !check_role(SERVICE_ADMIN_ROLE, &user.roles) {
        return redirect("/na?required_role=crm_admin");
    }

    match open_frontend_html("assets/dist/app/managers.html").await {
        Ok(file) => file.into_response(&request),
        Err(FrontendAssetError::Read(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            HttpResponse::ServiceUnavailable()
                .body("CRM frontend assets are not built yet. Run `cd frontend && npm run build`.")
        }
        Err(error) => {
            log::error!("Failed to open CRM managers document: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/managers/add")]
/// Add a new manager record from the provided form data.
pub async fn add_manager(
    web::Form(form): web::Form<AddManagerForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let payload = match AddManagerPayload::try_from(form) {
        Ok(payload) => payload,
        Err(error) => {
            log::error!("Invalid add-manager data: {error}");
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&error));
        }
    };

    match managers_service::add_manager(payload, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Created().json(ApiMutationSuccessDto {
            message: "Менеджер добавлен.".to_string(),
            redirect_to: None,
        }),
        Err(err) => {
            log::error!("Failed to save the manager: {err}");
            mutation_error_response(MutationResource::Manager, &err)
        }
    }
}

#[post("/managers/assign")]
/// Assign a manager to multiple clients based on submitted payload.
pub async fn assign_manager(
    payload: web::Bytes,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let form: AssignManagerForm = match serde_html_form::from_bytes(&payload) {
        Ok(form) => form,
        Err(err) => {
            log::error!("Failed to process form: {err}");
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::default());
        }
    };

    let payload = match AssignManagerPayload::try_from(form) {
        Ok(payload) => payload,
        Err(error) => {
            log::error!("Invalid assign-manager data: {error}");
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&error));
        }
    };

    match managers_service::assign_manager(payload, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Менеджер назначен клиентам.".to_string(),
            redirect_to: None,
        }),
        Err(err) => {
            log::error!("Failed to assign clients to the manager: {err}");
            mutation_error_response(MutationResource::Manager, &err)
        }
    }
}
