use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use serde::Deserialize;

use crate::repository::DieselRepository;
use crate::services::{ServiceError, api as api_service};

#[derive(Deserialize)]
/// Query parameters accepted by the `/api/v1/clients` endpoint.
struct ApiV1ClientsQueryParams {
    /// Optional search query to filter clients.
    query: Option<String>,
    /// Optional page number for pagination.
    page: Option<usize>,
}

#[get("/v1/clients")]
/// Return a JSON list of clients with optional search and pagination.
///
/// Users without the `crm` role receive a `401 Unauthorized` response.
pub async fn api_v1_clients(
    params: web::Query<ApiV1ClientsQueryParams>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let query = api_service::ClientsQuery {
        search: params.query.clone(),
        page: params.page,
    };

    match api_service::list_clients(repo.get_ref(), &user, query) {
        Ok(response) => HttpResponse::Ok().json(response.clients),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list clients: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
