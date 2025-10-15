use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::{Context, Tera};

use crate::forms::managers::AddManagerForm;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, managers as managers_service};

#[get("/managers")]
pub async fn managers(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match managers_service::list_managers(repo.get_ref(), &user) {
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
pub async fn add_manager(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AddManagerForm>,
) -> impl Responder {
    match managers_service::add_manager(repo.get_ref(), &user, form) {
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
pub async fn managers_modal(
    manager_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match managers_service::load_manager_modal(repo.get_ref(), &user, manager_id.into_inner()) {
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
pub async fn assign_manager(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Bytes,
) -> impl Responder {
    match managers_service::assign_manager(repo.get_ref(), &user, form.as_ref()) {
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
