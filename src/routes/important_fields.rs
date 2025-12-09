//! Routes for managing important fields in the CRM.

use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::forms::important_fields::ImportantFieldsForm;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, important_fields as important_fields_service};

#[get("/important-fields")]
/// Show the list of configured important fields for the current user.
pub async fn show_important_fields(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match important_fields_service::load_important_fields(repo.get_ref(), &user) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "important_fields",
                &server_config.auth_service_url,
            );
            let fields_text = data.fields.join("\n");
            context.insert("fields_text", &fields_text);

            render_template(&tera, "important_fields/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to load important fields: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/important-fields")]
/// Save the posted list of important field names for the user.
pub async fn save_important_fields(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Form<ImportantFieldsForm>,
) -> impl Responder {
    match important_fields_service::save_important_fields(repo.get_ref(), &user, form.into_inner())
    {
        Ok(()) => {
            FlashMessage::success("Список полей обновлён.").send();
            redirect("/important-fields")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/important-fields")
        }
        Err(err) => {
            log::error!("Failed to save important fields: {err}");
            FlashMessage::error("Не удалось сохранить важные поля.").send();
            redirect("/important-fields")
        }
    }
}
