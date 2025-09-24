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
use crate::repository::{
    ClientEventListQuery, ClientEventReader, ClientEventWriter, ClientReader, ClientWriter,
    DieselRepository, ManagerWriter,
};

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

    if check_role("crm_manager", &user.roles)
        && !repo
            .check_client_assigned_to_manager(client_id, &user.email)
            .is_ok_and(|result| result)
    {
        FlashMessage::error("Этот клиент для вас не доступен").send();
        return redirect("/");
    }

    let client = match repo.get_client_by_id(client_id, user.hub_id) {
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

    let managers = match repo.list_managers(client_id) {
        Ok(managers) => managers,
        Err(e) => {
            log::error!("Failed to get managers: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let events_with_managers = match repo.list_client_events(ClientEventListQuery::new(client_id)) {
        Ok((_total_events, events_with_managers)) => events_with_managers,
        Err(e) => {
            log::error!("Failed to get events: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let documents = match repo.list_client_events(
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

    let client = match repo.get_client_by_id(form.id, user.hub_id) {
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

    if check_role("crm_manager", &user.roles)
        && !repo
            .check_client_assigned_to_manager(client.id, &user.email)
            .is_ok_and(|result| result)
    {
        FlashMessage::error("Этот клиент для вас не доступен").send();
        return redirect("/");
    }
    let updates: UpdateClient = form.into();

    match repo.update_client(client.id, &updates) {
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

    if check_role("crm_manager", &user.roles)
        && !repo
            .check_client_assigned_to_manager(form.id, &user.email)
            .is_ok_and(|result| result)
    {
        FlashMessage::error("Этот клиент для вас не доступен").send();
        return redirect("/");
    }

    if let Err(e) = form.validate() {
        log::error!("Failed to validate form: {e}");
        FlashMessage::error("Ошибка валидации формы").send();
        return redirect(&format!("/client/{}", form.id));
    }

    let manager = match repo.create_or_update_manager(&(&user).into()) {
        Ok(manager) => manager,
        Err(err) => {
            log::error!("Failed to create or update manager: {err}");
            FlashMessage::error("Ошибка при добавлении комментария.").send();
            return redirect(&format!("/client/{}", form.id));
        }
    };

    let client = match repo.get_client_by_id(form.id, user.hub_id) {
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
            message: form.message.clone(),
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
        "text": &form.message,
    });

    if let Some(subject) = form.subject.as_ref() {
        if !subject.is_empty() {
            event_data["subject"] = json!(subject);
        }
    }

    let updates = NewClientEvent {
        client_id: client.id,
        event_type: ClientEventType::from(form.event_type.as_str()),
        manager_id: manager.id,
        created_at: Utc::now().naive_utc(),
        event_data,
    };

    match repo.create_client_event(&updates) {
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

    let manager = match repo.create_or_update_manager(&(&user).into()) {
        Ok(manager) => manager,
        Err(err) => {
            log::error!("Failed to create or update manager: {err}");
            FlashMessage::error("Ошибка при добавлении вложения.").send();
            return redirect(&format!("/client/{}", form.id));
        }
    };

    let client = match repo.get_client_by_id(form.id, user.hub_id) {
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

    if check_role("crm_manager", &user.roles)
        && !repo
            .check_client_assigned_to_manager(form.id, &user.email)
            .is_ok_and(|result| result)
    {
        FlashMessage::error("Этот клиент для вас не доступен").send();
        return redirect("/");
    }

    match repo.create_client_event(&updates) {
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
