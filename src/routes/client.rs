use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use chrono::Utc;
use log::error;
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{alert_level_to_str, check_role, ensure_role, redirect};
use serde_json::json;
use tera::Context;
use validator::Validate;

use crate::domain::client::UpdateClient;
use crate::domain::client_event::{ClientEventType, NewClientEvent};
use crate::forms::client::{AddAttachmentForm, AddCommentForm, SaveClientForm};
use crate::repository::client::DieselClientRepository;
use crate::repository::client_event::DieselClientEventRepository;
use crate::repository::manager::DieselManagerRepository;
use crate::repository::{
    ClientEventListQuery, ClientEventReader, ClientEventWriter, ClientReader, ClientWriter,
    ManagerWriter,
};
use crate::routes::render_template;

#[get("/client/{client_id}")]
pub async fn show_client(
    client_id: web::Path<i32>,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
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

    let event_repo = DieselClientEventRepository::new(&pool);
    let events_with_managers = match event_repo.list(ClientEventListQuery::new(client_id)) {
        Ok((_total_events, events_with_managers)) => events_with_managers,
        Err(e) => {
            error!("Failed to get events: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let documents = match event_repo
        .list(ClientEventListQuery::new(client_id).event_type(ClientEventType::DocumentLink))
    {
        Ok((_total_events, events_with_managers)) => events_with_managers
            .into_iter()
            .map(|(documents, _manager)| documents)
            .collect::<Vec<_>>(),
        Err(e) => {
            error!("Failed to get events: {e}");
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
    context.insert("events", &events_with_managers);
    context.insert("documents", &documents);

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

    if let Err(e) = form.validate() {
        FlashMessage::error(format!("Ошибка валидации формы: {e}")).send();
        return redirect(&format!("/client/{}", form.id));
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

#[post("/client/comment")]
pub async fn comment_client(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddCommentForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    }

    if let Err(e) = form.validate() {
        FlashMessage::error(format!("Ошибка валидации формы: {e}")).send();
        return redirect(&format!("/client/{}", form.id));
    }

    let manager_repo = DieselManagerRepository::new(&pool);

    let manager = match manager_repo.create_or_update(&(&user).into()) {
        Ok(manager) => manager,
        Err(err) => {
            error!("Failed to create or update manager: {err}");
            FlashMessage::error("Ошибка при добавлении комментария.").send();
            return redirect(&format!("/client/{}", form.id));
        }
    };

    let client_repo = DieselClientRepository::new(&pool);
    let updates = NewClientEvent {
        client_id: form.id,
        event_type: ClientEventType::from(form.event_type.as_str()),
        manager_id: manager.id,
        created_at: Utc::now().naive_utc(),
        event_data: json!({
            "text": &form.text,
        }),
    };

    if check_role("crm_manager", &user.roles)
        && !client_repo
            .check_manager_assigned(form.id, &user.email)
            .is_ok_and(|result| result)
    {
        FlashMessage::error("Этот клиент для вас не доступен").send();
        return redirect("/");
    }

    let client_event_repo = DieselClientEventRepository::new(&pool);

    match client_event_repo.create(&updates) {
        Ok(_) => {
            FlashMessage::success("Событие добавлено.".to_string()).send();
        }
        Err(err) => {
            error!("Failed to update client: {err}");
            FlashMessage::error(format!("Ошибка при добавлении события: {err}")).send();
        }
    }

    redirect(&format!("/client/{}", form.id))
}

#[post("/client/attachment")]
pub async fn attachment_client(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddAttachmentForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    }

    if let Err(e) = form.validate() {
        FlashMessage::error(format!("Ошибка валидации формы: {e}")).send();
        return redirect(&format!("/client/{}", form.id));
    }

    let manager_repo = DieselManagerRepository::new(&pool);

    let manager = match manager_repo.create_or_update(&(&user).into()) {
        Ok(manager) => manager,
        Err(err) => {
            error!("Failed to create or update manager: {err}");
            FlashMessage::error("Ошибка при добавлении вложения.").send();
            return redirect(&format!("/client/{}", form.id));
        }
    };

    let client_repo = DieselClientRepository::new(&pool);
    let updates = NewClientEvent {
        client_id: form.id,
        event_type: ClientEventType::DocumentLink,
        manager_id: manager.id,
        created_at: Utc::now().naive_utc(),
        event_data: json!({
            "text": &form.text,
            "url": &form.url,
        }),
    };

    if check_role("crm_manager", &user.roles)
        && !client_repo
            .check_manager_assigned(form.id, &user.email)
            .is_ok_and(|result| result)
    {
        FlashMessage::error("Этот клиент для вас не доступен").send();
        return redirect("/");
    }

    let client_event_repo = DieselClientEventRepository::new(&pool);

    match client_event_repo.create(&updates) {
        Ok(_) => {
            FlashMessage::success("Событие добавлено.".to_string()).send();
        }
        Err(err) => {
            error!("Failed to update client: {err}");
            FlashMessage::error(format!("Ошибка при добавлении события: {err}")).send();
        }
    }

    redirect(&format!("/client/{}", form.id))
}
