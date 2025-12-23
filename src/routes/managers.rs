//! Routes that manage manager assignments.

use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::{Context, Tera};

use crate::forms::managers::{AddManagerForm, AssignManagerForm};
use crate::repository::DieselRepository;
use crate::services::{ServiceError, managers as managers_service};

#[get("/managers")]
/// Render the managers list page, showing assignments and controls.
pub async fn managers(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match managers_service::list_managers(&user, repo.get_ref()) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "settings",
                &server_config.auth_service_url,
            );
            context.insert("managers", &data.managers);

            render_template(&tera, "managers/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to list managers: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/managers/add")]
/// Add a new manager record from the provided form data.
pub async fn add_manager(
    web::Form(form): web::Form<AddManagerForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match managers_service::add_manager(form, &user, repo.get_ref()) {
        Ok(()) => {
            FlashMessage::success("Менеджер добавлен.").send();
            redirect("/managers")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/managers")
        }
        Err(err) => {
            log::error!("Failed to save the manager: {err}");
            FlashMessage::error("Ошибка при добавлении менеджера").send();
            redirect("/managers")
        }
    }
}

#[post("/managers/modal/{manager_id}")]
/// Return the modal body with manager details for the assignee dialog.
pub async fn managers_modal(
    manager_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match managers_service::load_manager_modal(manager_id.into_inner(), &user, repo.get_ref()) {
        Ok(data) => {
            let mut context = Context::new();
            context.insert("manager", &data.manager);
            context.insert("clients", &data.clients);
            render_template(&tera, "managers/modal_body.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            log::error!("Unauthorized to load manager modal.");
            HttpResponse::Unauthorized().finish()
        }
        Err(ServiceError::NotFound) => HttpResponse::InternalServerError().finish(),
        Err(err) => {
            log::error!("Failed to load manager modal: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/managers/assign")]
/// Assign a manager to multiple clients based on submitted payload.
pub async fn assign_manager(
    payload: web::Bytes,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let form: AssignManagerForm = match serde_html_form::from_bytes(&payload) {
        Ok(form) => form,
        Err(err) => {
            log::error!("Failed to process form: {err}");
            FlashMessage::error("Ошибка при обработке формы").send();
            return redirect("/managers");
        }
    };

    match managers_service::assign_manager(form, &user, repo.get_ref()) {
        Ok(()) => {
            FlashMessage::success("Менеджер назначен клиентам.").send();
            redirect("/managers")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/managers")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Менеджер не найден.").send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to assign clients to the manager: {err}");
            FlashMessage::error("Ошибка при назначении клиентов менеджера").send();
            redirect("/managers")
        }
    }
}
