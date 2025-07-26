use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::routes::{check_role, ensure_role};
use serde::Deserialize;

use crate::repository::client::DieselClientRepository;
use crate::repository::{ClientListQuery, ClientReader};

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

    let mut search_params = ClientListQuery::new(user.hub_id);

    if check_role("crm_manager", user.roles) {
        search_params = search_params.manager_email(&user.email);
    }

    if let Some(page) = params.page {
        search_params = search_params.paginate(page, DEFAULT_ITEMS_PER_PAGE);
    }

    let results = match &params.query {
        Some(query) if !query.is_empty() => {
            search_params = search_params.search(query);
            repo.search(search_params)
        }
        _ => repo.list(search_params)
    };

    match results {
        Ok((_total, clients)) => HttpResponse::Ok().json(clients),
        Err(e) => {
            log::error!("Failed to list clients: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
