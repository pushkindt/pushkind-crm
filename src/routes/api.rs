//! Actix routes serving the CRM API surface.

use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use serde::Deserialize;

use crate::dto::api::ClientsQuery;
use crate::dto::main::IndexQuery;
use crate::models::config::AppConfig;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, api as api_service};

#[derive(Debug, Default, Deserialize)]
pub struct NoAccessQuery {
    pub required_role: Option<String>,
}

#[get("/v1/iam")]
/// Return typed shell data for React-owned CRM pages.
pub async fn api_v1_iam(
    user: AuthenticatedUser,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    match api_service::get_shell_data(&user, common_config.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load CRM shell data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/client-directory")]
/// Return typed client directory data.
pub async fn api_v1_client_directory(
    params: web::Query<IndexQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_client_directory_data(params.into_inner(), &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load CRM client directory data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/clients/{client_id}")]
/// Return typed client details data.
pub async fn api_v1_client_details(
    client_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    match api_service::get_client_details_data(
        client_id.into_inner(),
        &user,
        repo.get_ref(),
        app_config.get_ref(),
    ) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to load CRM client details data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/managers")]
/// Return typed manager collection data.
pub async fn api_v1_managers(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_manager_collection_data(&user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load CRM managers page data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/managers/{manager_id}")]
/// Return typed manager modal data for React-owned CRM pages.
pub async fn api_v1_manager_modal(
    manager_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_manager_modal_data(manager_id.into_inner(), &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to load CRM manager modal data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/important-fields")]
/// Return typed important-field settings data.
pub async fn api_v1_important_fields(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_important_field_settings_data(&user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load CRM important-field settings data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/no-access")]
/// Return page data for the CRM no-access page.
pub async fn api_v1_no_access(
    query: web::Query<NoAccessQuery>,
    user: AuthenticatedUser,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    HttpResponse::Ok().json(api_service::get_no_access_data(
        &user,
        common_config.get_ref(),
        query.required_role.as_deref(),
    ))
}

#[get("/v1/clients")]
/// Return a JSON list of clients with optional search and pagination.
///
/// Users without either the `crm` or `crm_admin` role receive a
/// `401 Unauthorized` response.
pub async fn api_v1_clients(
    params: web::Query<ClientsQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::list_clients(params.into_inner(), &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response.clients),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list clients: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
