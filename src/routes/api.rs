use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::routes::{DEFAULT_ITEMS_PER_PAGE, ensure_role};
use serde::Deserialize;

use crate::repository::client::DieselClientRepository;
use crate::repository::{ClientListQuery, ClientReader, ClientSearchQuery};

#[derive(Deserialize)]
struct ApiV1ClientsQueryParams {
    query: Option<String>,
    page: Option<usize>,
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

    match &params.query {
        Some(query) if !query.is_empty() => {
            let mut search_params = ClientSearchQuery::new(user.hub_id, query);
            if let Some(page) = params.page {
                search_params = search_params.paginate(page, DEFAULT_ITEMS_PER_PAGE);
            }

            match repo.search(search_params) {
                Ok((_total, clients)) => HttpResponse::Ok().json(clients),
                Err(e) => {
                    log::error!("Failed to list clients: {e}");
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        _ => {
            let mut list_params = ClientListQuery::new(user.hub_id);
            if let Some(page) = params.page {
                list_params = list_params.paginate(page, DEFAULT_ITEMS_PER_PAGE);
            }

            match repo.list(list_params) {
                Ok((_total, clients)) => HttpResponse::Ok().json(clients),
                Err(e) => {
                    log::error!("Failed to list clients: {e}");
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
    }
}
