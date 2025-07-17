use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use log::error;
use tera::Context;

use crate::db::DbPool;
use crate::domain::client::UpdateClient;
use crate::forms::client::SaveClientForm;
use crate::models::auth::AuthenticatedUser;
use crate::models::config::ServerConfig;
use crate::repository::client::DieselClientRepository;
use crate::repository::{ClientReader, ClientWriter};
use crate::routes::{alert_level_to_str, check_role, ensure_role, redirect, render_template};

#[get("/client/{client_id}")]
pub async fn show_client(
    client_id: web::Path<i32>,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    };

    let client_repo = DieselClientRepository::new(&pool);

    let client_id = client_id.into_inner();

    if check_role("crm_manager", &user.roles)
        && !client_repo
            .check_manager_assigned(client_id, &user.email)
            .is_ok_and(|result| result)
    {
        FlashMessage::error("Этот клиент для вас не доступен").send();
        return redirect("/");
    }

    let client = match client_repo.get_by_id(client_id) {
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

    let managers = match client_repo.list_managers(client_id) {
        Ok(managers) => managers,
        Err(e) => {
            error!("Failed to get managers: {e}");
            return HttpResponse::InternalServerError().finish();
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
    context.insert("managers", &managers);

    render_template("client/index.html", &context)
}

#[post("/client/save")]
pub async fn save_client(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<SaveClientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    }

    let client_repo = DieselClientRepository::new(&pool);
    let updates: UpdateClient = (&form).into();

    if check_role("crm_manager", &user.roles)
        && !client_repo
            .check_manager_assigned(form.id, &user.email)
            .is_ok_and(|result| result)
    {
        FlashMessage::error("Этот клиент для вас не доступен").send();
        return redirect("/");
    }

    match client_repo.update(form.id, &updates) {
        Ok(_) => {
            FlashMessage::success("Клиент обновлен.".to_string()).send();
        }
        Err(err) => {
            error!("Failed to update client: {err}");
            FlashMessage::error(format!("Ошибка при обновлении клиента: {err}")).send();
        }
    }

    redirect(&format!("/client/{}", form.id))
}
