use actix_web::{HttpResponse, Responder, get, web};
use log::error;
use serde::Deserialize;

use crate::db::DbPool;
use crate::models::auth::AuthenticatedUser;
use crate::repository::ClientRepository;
use crate::repository::client::DieselClientRepository;
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
    if let Err(_) = ensure_role(&user, "crm", Some("/na")) {
        return HttpResponse::Unauthorized().finish();
    }
    let repo = DieselClientRepository::new(&pool);

    match repo.search(user.hub_id, &params.query) {
        Ok(clients) => HttpResponse::Ok().json(clients),
        Err(e) => {
            error!("Failed to list clients: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
