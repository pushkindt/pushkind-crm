use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, render_template};
use pushkind_common::routes::{ensure_role, redirect};
use tera::{Context, Tera};
use validator::Validate;

use crate::domain::manager::NewManager;
use crate::forms::managers::{AddManagerForm, AssignManagerForm};
use crate::repository::{
    ClientListQuery, ClientReader, DieselRepository, ManagerReader, ManagerWriter,
};

#[get("/managers")]
pub async fn managers(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm_admin", Some("/na")) {
        return response;
    };

    let managers = match repo.list_managers_with_clients(user.hub_id) {
        Ok(managers) => managers,
        Err(err) => {
            log::error!("Failed to list managers: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "settings",
        &server_config.auth_service_url,
    );
    context.insert("managers", &managers);

    render_template(&tera, "managers/index.html", &context)
}

#[post("/managers/add")]
pub async fn add_manager(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AddManagerForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm_admin", Some("/na")) {
        return response;
    };

    if let Err(e) = form.validate() {
        log::error!("Failed to validate form: {e}");
        FlashMessage::error("Ошибка валидации формы").send();
        return redirect("/managers");
    }

    let new_manager = NewManager {
        hub_id: user.hub_id,
        name: &form.name,
        email: &form.email,
    };

    match repo.create_or_update_manager(&new_manager) {
        Ok(_) => {
            FlashMessage::success("Менеджер добавлен.".to_string()).send();
        }
        Err(err) => {
            log::error!("Failed to save the manager: {err}");
            FlashMessage::error("Ошибка при добавлении менеджера").send();
        }
    }
    redirect("/managers")
}

#[post("/managers/modal/{manager_id}")]
pub async fn managers_modal(
    manager_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm_admin", Some("/na")) {
        return response;
    };

    let mut context = Context::new();

    let manager_id = manager_id.into_inner();

    let manager = match repo.get_manager_by_id(manager_id, user.hub_id) {
        Ok(Some(manager)) => manager,
        _ => return HttpResponse::InternalServerError().finish(),
    };

    context.insert("manager", &manager);

    let clients =
        match repo.list_clients(ClientListQuery::new(user.hub_id).manager_email(&manager.email)) {
            Ok((_total, clients)) => clients,
            Err(err) => {
                log::error!("Failed to list clients: {err}");
                return HttpResponse::InternalServerError().finish();
            }
        };

    context.insert("clients", &clients);

    render_template(&tera, "managers/modal_body.html", &context)
}

#[post("/managers/assign")]
pub async fn assign_manager(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Bytes,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm_admin", Some("/na")) {
        return response;
    };

    let form: AssignManagerForm = match serde_html_form::from_bytes(&form) {
        Ok(form) => form,
        Err(err) => {
            log::error!("Failed to process form: {err}");
            FlashMessage::error("Ошибка при обработке формы").send();
            return redirect("/managers");
        }
    };

    let manager = match repo.get_manager_by_id(form.manager_id, user.hub_id) {
        Ok(Some(manager)) => manager,
        Err(e) => {
            log::error!("Failed to get manager: {e}");
            return HttpResponse::InternalServerError().finish();
        }
        _ => {
            FlashMessage::error("Менеджер не найден.").send();
            return redirect("/");
        }
    };

    match repo.assign_clients_to_manager(manager.id, &form.client_ids) {
        Ok(_) => {
            FlashMessage::success("Менеджер назначен клиентам.".to_string()).send();
        }
        Err(err) => {
            log::error!("Failed to assign clients to the manager: {err}");
            FlashMessage::error("Ошибка при назначении клиентов менеджера").send();
        }
    }
    redirect("/managers")
}
