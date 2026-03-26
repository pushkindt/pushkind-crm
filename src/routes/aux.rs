use actix_web::{HttpRequest, HttpResponse, get};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::frontend::{FrontendAssetError, open_frontend_html};

#[get("/na")]
pub async fn not_assigned(request: HttpRequest, _user: AuthenticatedUser) -> HttpResponse {
    match open_frontend_html("assets/dist/app/no-access.html").await {
        Ok(file) => file.into_response(&request),
        Err(FrontendAssetError::Read(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            HttpResponse::ServiceUnavailable()
                .body("CRM frontend assets are not built yet. Run `cd frontend && npm run build`.")
        }
        Err(error) => {
            log::error!("Failed to open CRM no-access document: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
