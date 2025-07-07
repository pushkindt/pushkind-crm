use actix_web::{HttpResponse, Responder, get, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use log::error;
use tera::Context;

use crate::models::auth::AuthenticatedUser;
use crate::models::config::ServerConfig;
use crate::repository::ClientRepository;
use crate::repository::test::TestClientRepository;
use crate::routes::{alert_level_to_str, ensure_role, redirect, render_template};

#[get("/client/{client_id}")]
pub async fn client(
    client_id: web::Path<i32>,
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    };

    let repo = TestClientRepository;

    let client = match repo.get_by_id(client_id.into_inner()) {
        Ok(Some(client)) if client.hub_id == user.hub_id => client,
        Err(e) => {
            error!("Failed to get client: {e}");
            return HttpResponse::InternalServerError().finish();
        }
        _ => {
            FlashMessage::error("Клиент не найден.").send();
            return redirect("/");
        }
    };

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "client");
    context.insert("home_url", &server_config.auth_service_url);
    context.insert("client", &client);

    render_template("client/index.html", &context)
}
