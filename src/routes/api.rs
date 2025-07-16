use actix_web::{HttpResponse, Responder, get, web};
use log::error;
use serde::Deserialize;

use crate::db::DbPool;
use crate::models::auth::AuthenticatedUser;
use crate::repository::client::DieselClientRepository;
use crate::repository::{ClientReader, ClientSearchQuery};
use crate::routes::ensure_role;

#[derive(Deserialize)]
struct ApiV1ClientsQueryParams {
    query: String,
}

#[get("/v1/clients")]
pub async fn api_v1_clients(
    params: web::Query<ApiV1ClientsQueryParams>,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
) -> impl Responder {
    if ensure_role(&user, "crm", Some("/na")).is_err() {
        return HttpResponse::Unauthorized().finish();
    }
    let repo = DieselClientRepository::new(&pool);

    match repo.search(ClientSearchQuery::new(user.hub_id, &params.query)) {
        Ok((_total, clients)) => HttpResponse::Ok().json(clients),
        Err(e) => {
            error!("Failed to list clients: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
