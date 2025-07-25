use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use log::error;
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{alert_level_to_str, ensure_role, redirect};
use tera::Context;
use validator::Validate;

use crate::domain::manager::NewManager;
use crate::forms::managers::{AddManagerForm, AssignManagerForm};
use crate::repository::client::DieselClientRepository;
use crate::repository::manager::DieselManagerRepository;
use crate::repository::{ClientListQuery, ClientReader, ManagerReader, ManagerWriter};
use crate::routes::render_template;

#[get("/managers")]
pub async fn managers(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm_admin", Some("/na")) {
        return response;
    };

    let repo = DieselManagerRepository::new(&pool);

    let managers = match repo.list(user.hub_id) {
        Ok(managers) => managers,
        Err(err) => {
            error!("Failed to list managers: {err}");
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
    context.insert("current_page", "settings");
    context.insert("home_url", &server_config.auth_service_url);
    context.insert("managers", &managers);

    render_template("managers/index.html", &context)
}

#[post("/managers/add")]
pub async fn add_manager(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddManagerForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm_admin", Some("/na")) {
        return response;
    };

    if let Err(e) = form.validate() {
        error!("Failed to validate form: {e}");
        FlashMessage::error("Ошибка валидации формы").send();
        return redirect("/managers");
    }

    let new_manager = NewManager {
        hub_id: user.hub_id,
        name: &form.name,
        email: &form.email,
    };

    let repo = DieselManagerRepository::new(&pool);
    match repo.create_or_update(&new_manager) {
        Ok(_) => {
            FlashMessage::success("Менеджер добавлен.".to_string()).send();
        }
        Err(err) => {
            error!("Failed to save the manager: {err}");
            FlashMessage::error("Ошибка при добавлении менеджера").send();
        }
    }
    redirect("/managers")
}

#[post("/managers/modal/{manager_id}")]
pub async fn managers_modal(
    manager_id: web::Path<i32>,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm_admin", Some("/na")) {
        return response;
    };

    let manager_repo = DieselManagerRepository::new(&pool);

    let mut context = Context::new();

    let manager_id = manager_id.into_inner();

    let manager = match manager_repo.get_by_id(manager_id) {
        Ok(Some(manager)) => manager,
        _ => return HttpResponse::InternalServerError().finish(),
    };

    context.insert("manager", &manager);
    let client_repo = DieselClientRepository::new(&pool);

    let clients =
        match client_repo.list(ClientListQuery::new(user.hub_id).manager_email(&manager.email)) {
            Ok((_total, clients)) => clients,
            Err(err) => {
                error!("Failed to list clients: {err}");
                return HttpResponse::InternalServerError().finish();
            }
        };

    context.insert("clients", &clients);

    render_template("managers/modal_body.html", &context)
}

#[post("/managers/assign")]
pub async fn assign_manager(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    form: web::Bytes,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm_admin", Some("/na")) {
        return response;
    };

    let form: AssignManagerForm = match serde_html_form::from_bytes(&form) {
        Ok(form) => form,
        Err(err) => {
            error!("Failed to process form: {err}");
            FlashMessage::error("Ошибка при обработке формы").send();
            return redirect("/managers");
        }
    };

    let repo = DieselManagerRepository::new(&pool);
    match repo.assign_clients(form.manager_id, &form.client_ids) {
        Ok(_) => {
            FlashMessage::success("Менеджер назначен клиентам.".to_string()).send();
        }
        Err(err) => {
            error!("Failed to assign clients to the manager: {err}");
            FlashMessage::error("Ошибка при назначении клиентов менеджера").send();
        }
    }
    redirect("/managers")
}
