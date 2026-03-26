//! Routes for managing important fields in the CRM.

use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::{check_role, redirect};

use crate::SERVICE_ADMIN_ROLE;
use crate::dto::api::{ApiMutationErrorDto, ApiMutationSuccessDto};
use crate::forms::important_fields::{ImportantFieldsForm, ImportantFieldsPayload};
use crate::frontend::{FrontendAssetError, open_frontend_html};
use crate::repository::DieselRepository;
use crate::routes::{MutationResource, mutation_error_response};
use crate::services::settings as important_fields_service;

#[get("/settings")]
/// Show the list of configured important fields for the current user.
pub async fn show_settings(
    request: HttpRequest,
    user: AuthenticatedUser,
    _repo: web::Data<DieselRepository>,
) -> impl Responder {
    if !check_role(SERVICE_ADMIN_ROLE, &user.roles) {
        return redirect("/na");
    }

    match open_frontend_html("assets/dist/app/settings.html").await {
        Ok(file) => file.into_response(&request),
        Err(FrontendAssetError::Read(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            HttpResponse::ServiceUnavailable()
                .body("CRM frontend assets are not built yet. Run `cd frontend && npm run build`.")
        }
        Err(error) => {
            log::error!("Failed to open CRM settings document: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/important-fields")]
/// Save the posted list of important field names for the user.
pub async fn save_important_fields(
    form: web::Form<ImportantFieldsForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let payload = match ImportantFieldsPayload::try_from(form.into_inner()) {
        Ok(payload) => payload,
        Err(error) => {
            log::error!("Invalid important fields data: {error}");
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&error));
        }
    };

    match important_fields_service::save_important_fields(payload, &user, repo.get_ref()) {
        Ok(()) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Список полей обновлён.".to_string(),
            redirect_to: None,
        }),
        Err(err) => {
            log::error!("Failed to save important fields: {err}");
            mutation_error_response(MutationResource::Settings, &err)
        }
    }
}

#[post("/settings/cleanup")]
/// Remove all clients and related data for the current hub.
pub async fn cleanup_clients(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match important_fields_service::cleanup_clients(&user, repo.get_ref()) {
        Ok(()) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Клиенты очищены.".to_string(),
            redirect_to: None,
        }),
        Err(err) => {
            log::error!("Failed to cleanup clients: {err}");
            mutation_error_response(MutationResource::Settings, &err)
        }
    }
}
