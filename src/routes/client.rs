use std::collections::HashMap;
use std::sync::Arc;

use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use chrono::Utc;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::domain::emailer::email::{NewEmail, NewEmailRecipient};
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::models::emailer::zmq::ZMQSendEmailMessage;
use pushkind_common::routes::{base_context, render_template};
use pushkind_common::routes::{check_role, ensure_role, redirect};
use pushkind_common::zmq::ZmqSender;
use serde_json::json;
use tera::Tera;
use validator::Validate;

use crate::domain::client::UpdateClient;
use crate::domain::client_event::{ClientEventType, NewClientEvent};
use crate::forms::client::{AddAttachmentForm, AddCommentForm, SaveClientForm};
use crate::repository::{ClientEventListQuery, DieselRepository};
use crate::services::client as client_service;

#[get("/client/{client_id}")]
pub async fn show_client(
    client_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    };

    let client_id = client_id.into_inner();
    let repo = repo.get_ref();

    if check_role("crm_manager", &user.roles) {
        match client_service::is_client_assigned_to_manager(repo, client_id, &user.email) {
            Ok(true) => {}
            Ok(false) => {
                FlashMessage::error("Этот клиент для вас не доступен").send();
                return redirect("/");
            }
            Err(e) => {
                log::error!("Failed to check client access: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        }
    }

    let client = match client_service::get_client_by_id(repo, client_id, user.hub_id) {
        Ok(Some(client)) => client,
        Err(e) => {
            log::error!("Failed to get client: {e}");
            return HttpResponse::InternalServerError().finish();
        }
        _ => {
            FlashMessage::error("Клиент не найден.").send();
            return redirect("/");
        }
    };

    let managers = match client_service::list_client_managers(repo, client_id) {
        Ok(managers) => managers,
        Err(e) => {
            log::error!("Failed to get managers: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let events_with_managers =
        match client_service::list_client_events(repo, ClientEventListQuery::new(client_id)) {
            Ok((_total_events, events_with_managers)) => events_with_managers,
            Err(e) => {
                log::error!("Failed to get events: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };
    let documents = match client_service::list_client_events(
        repo,
        ClientEventListQuery::new(client_id).event_type(ClientEventType::DocumentLink),
    ) {
        Ok((_total_events, events_with_managers)) => events_with_managers
            .into_iter()
            .map(|(documents, _manager)| documents)
            .collect::<Vec<_>>(),
        Err(e) => {
            log::error!("Failed to get events: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "index",
        &server_config.auth_service_url,
    );
    context.insert("client", &client);
    context.insert("managers", &managers);
    context.insert("events", &events_with_managers);
    context.insert("documents", &documents);

    render_template(&tera, "client/index.html", &context)
}

#[post("/client/save")]
pub async fn save_client(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Bytes,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    }

    let form: SaveClientForm = match serde_html_form::from_bytes(&form) {
        Ok(form) => form,
        Err(err) => {
            log::error!("Error parsing form: {err}");
            FlashMessage::error("Ошибка при обработке формы.").send();
            return redirect("/clients");
        }
    };

    if let Err(e) = form.validate() {
        log::error!("Failed to validate form: {e}");
        FlashMessage::error("Ошибка валидации формы").send();
        return redirect(&format!("/client/{}", form.id));
    }

    let repo = repo.get_ref();

    let client = match client_service::get_client_by_id(repo, form.id, user.hub_id) {
        Ok(Some(client)) => client,
        Err(e) => {
            log::error!("Failed to get client: {e}");
            return HttpResponse::InternalServerError().finish();
        }
        _ => {
            FlashMessage::error("Клиент не найден.").send();
            return redirect("/");
        }
    };

    if check_role("crm_manager", &user.roles) {
        match client_service::is_client_assigned_to_manager(repo, client.id, &user.email) {
            Ok(true) => {}
            Ok(false) => {
                FlashMessage::error("Этот клиент для вас не доступен").send();
                return redirect("/");
            }
            Err(e) => {
                log::error!("Failed to check client access: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        }
    }
    let updates: UpdateClient = form.into();

    match client_service::update_client(repo, client.id, &updates) {
        Ok(_) => {
            FlashMessage::success("Клиент обновлен.".to_string()).send();
        }
        Err(err) => {
            log::error!("Failed to update client: {err}");
            FlashMessage::error("Ошибка при обновлении клиента").send();
        }
    }

    redirect(&format!("/client/{}", client.id))
}

#[post("/client/comment")]
pub async fn comment_client(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
    web::Form(form): web::Form<AddCommentForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    }

    let repo = repo.get_ref();

    if check_role("crm_manager", &user.roles) {
        match client_service::is_client_assigned_to_manager(repo, form.id, &user.email) {
            Ok(true) => {}
            Ok(false) => {
                FlashMessage::error("Этот клиент для вас не доступен").send();
                return redirect("/");
            }
            Err(e) => {
                log::error!("Failed to check client access: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        }
    }

    if let Err(e) = form.validate() {
        log::error!("Failed to validate form: {e}");
        FlashMessage::error("Ошибка валидации формы").send();
        return redirect(&format!("/client/{}", form.id));
    }

    let sanitized_message = ammonia::clean(&form.message);

    let manager = match client_service::create_or_update_manager(repo, &(&user).into()) {
        Ok(manager) => manager,
        Err(err) => {
            log::error!("Failed to create or update manager: {err}");
            FlashMessage::error("Ошибка при добавлении комментария.").send();
            return redirect(&format!("/client/{}", form.id));
        }
    };

    let client = match client_service::get_client_by_id(repo, form.id, user.hub_id) {
        Ok(Some(client)) => client,
        Err(e) => {
            log::error!("Failed to get client: {e}");
            return HttpResponse::InternalServerError().finish();
        }
        _ => {
            FlashMessage::error("Клиент не найден.").send();
            return redirect("/");
        }
    };

    if form.event_type == "Email" {
        let client_email = match client.email.as_ref() {
            Some(email) => email,
            None => {
                FlashMessage::error("Клиент не имеет email").send();
                return redirect(&format!("/client/{}", form.id));
            }
        };

        let new_email = NewEmail {
            message: sanitized_message.clone(),
            subject: form.subject.clone(),
            attachment: None,
            attachment_name: None,
            attachment_mime: None,
            hub_id: user.hub_id,
            recipients: vec![NewEmailRecipient {
                address: client_email.clone(),
                name: client.name.clone(),
                fields: client.fields.map(HashMap::from_iter).unwrap_or_default(),
            }],
        };

        let zmq_message = ZMQSendEmailMessage::NewEmail(Box::new((user.clone(), new_email)));

        if let Err(err) = zmq_sender.send_json(&zmq_message).await {
            log::error!("Ошибка при добавлении сообщения в очередь: {err}");
            FlashMessage::error("Ошибка при добавлении сообщения в очередь.").send();
            return redirect(&format!("/client/{}", form.id));
        }
    }

    let mut event_data = json!({
        "text": sanitized_message,
    });

    if let Some(subject) = form.subject.as_ref() {
        event_data["subject"] = json!(subject);
    }

    let updates = NewClientEvent {
        client_id: client.id,
        event_type: ClientEventType::from(form.event_type.as_str()),
        manager_id: manager.id,
        created_at: Utc::now().naive_utc(),
        event_data,
    };

    match client_service::create_client_event(repo, &updates) {
        Ok(_) => {
            FlashMessage::success("Событие добавлено.".to_string()).send();
        }
        Err(err) => {
            log::error!("Failed to update client: {err}");
            FlashMessage::error("Ошибка при добавлении события").send();
        }
    }

    redirect(&format!("/client/{}", form.id))
}

#[post("/client/attachment")]
pub async fn attachment_client(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AddAttachmentForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "crm", Some("/na")) {
        return response;
    }

    if let Err(e) = form.validate() {
        log::error!("Failed to validate form: {e}");
        FlashMessage::error("Ошибка валидации формы").send();
        return redirect(&format!("/client/{}", form.id));
    }

    let repo = repo.get_ref();

    let manager = match client_service::create_or_update_manager(repo, &(&user).into()) {
        Ok(manager) => manager,
        Err(err) => {
            log::error!("Failed to create or update manager: {err}");
            FlashMessage::error("Ошибка при добавлении вложения.").send();
            return redirect(&format!("/client/{}", form.id));
        }
    };

    let client = match client_service::get_client_by_id(repo, form.id, user.hub_id) {
        Ok(Some(client)) => client,
        Err(e) => {
            log::error!("Failed to get client: {e}");
            return HttpResponse::InternalServerError().finish();
        }
        _ => {
            FlashMessage::error("Клиент не найден.").send();
            return redirect("/");
        }
    };

    let updates = NewClientEvent {
        client_id: client.id,
        event_type: ClientEventType::DocumentLink,
        manager_id: manager.id,
        created_at: Utc::now().naive_utc(),
        event_data: json!({
            "text": &form.text,
            "url": &form.url,
        }),
    };

    if check_role("crm_manager", &user.roles) {
        match client_service::is_client_assigned_to_manager(repo, form.id, &user.email) {
            Ok(true) => {}
            Ok(false) => {
                FlashMessage::error("Этот клиент для вас не доступен").send();
                return redirect("/");
            }
            Err(e) => {
                log::error!("Failed to check client access: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        }
    }

    match client_service::create_client_event(repo, &updates) {
        Ok(_) => {
            FlashMessage::success("Событие добавлено.".to_string()).send();
        }
        Err(err) => {
            log::error!("Failed to update client: {err}");
            FlashMessage::error("Ошибка при добавлении события").send();
        }
    }

    redirect(&format!("/client/{}", form.id))
}
