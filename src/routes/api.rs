//! Actix routes serving the CRM API surface.

use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::dto::api::ClientsQuery;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, api as api_service};

#[get("/v1/clients")]
/// Return a JSON list of clients with optional search and pagination.
///
/// Users without the `crm` role receive a `401 Unauthorized` response.
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
