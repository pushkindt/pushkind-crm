use std::collections::HashMap;

use chrono::Utc;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::domain::emailer::email::{NewEmail, NewEmailRecipient};
use pushkind_common::models::emailer::zmq::ZMQSendEmailMessage;
use pushkind_common::routes::check_role;
use pushkind_common::zmq::ZmqSender;
use serde::Serialize;
use serde_json::json;
use validator::Validate;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::client::{Client, NewClient, UpdateClient};
use crate::domain::client_event::{ClientEvent, ClientEventType, NewClientEvent};
use crate::domain::important_field::ImportantField;
use crate::domain::manager::{Manager, NewManager};
use crate::forms::client::{AddAttachmentForm, AddCommentForm, SaveClientForm};
use crate::repository::{
    ClientEventListQuery, ClientEventReader, ClientEventWriter, ClientReader, ClientWriter,
    ImportantFieldReader, ManagerReader, ManagerWriter,
};
use crate::services::{ServiceError, ServiceResult};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ClientFieldDisplay {
    pub label: String,
    pub value: Option<String>,
}

/// Aggregated data required to render the client details page.
#[derive(Debug)]
pub struct ClientPageData {
    pub client: Client,
    pub managers: Vec<Manager>,
    pub events_with_managers: Vec<(ClientEvent, Manager)>,
    pub documents: Vec<ClientEvent>,
    pub available_fields: Vec<String>,
    pub important_fields: Vec<ClientFieldDisplay>,
    pub other_fields: Vec<ClientFieldDisplay>,
    pub total_events: usize,
}

fn partition_client_fields(
    client: &Client,
    important_fields: &[ImportantField],
) -> (Vec<ClientFieldDisplay>, Vec<ClientFieldDisplay>) {
    let mut remaining_fields: Vec<(String, String)> = client
        .fields
        .as_ref()
        .map(|fields| {
            fields
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default();

    let mut important = Vec::with_capacity(important_fields.len());

    for field in important_fields {
        let label = field.field.trim();
        let normalized = label.to_lowercase();

        let value = remaining_fields
            .iter()
            .position(|(key, _)| key.trim().to_lowercase() == normalized)
            .map(|index| remaining_fields.remove(index).1)
            .and_then(normalize_field_value);

        important.push(ClientFieldDisplay {
            label: label.to_string(),
            value,
        });
    }

    let other = remaining_fields
        .into_iter()
        .map(|(label, value)| ClientFieldDisplay {
            label,
            value: normalize_field_value(value),
        })
        .collect();

    (important, other)
}

fn normalize_field_value(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn client_with_fields(fields: Vec<(&str, &str)>) -> Client {
        let mut map = BTreeMap::new();
        for (key, value) in fields {
            map.insert(key.to_string(), value.to_string());
        }

        Client {
            fields: if map.is_empty() { None } else { Some(map) },
            ..Client::default()
        }
    }

    #[test]
    fn partition_client_fields_extracts_configured_names() {
        let client = client_with_fields(vec![
            ("Favorite Color", "  Blue  "),
            (" Stage ", " In progress "),
            ("Other", "  "),
        ]);

        let configured = vec![
            ImportantField::new(1, "Stage".to_string()),
            ImportantField::new(1, "Missing".to_string()),
            ImportantField::new(1, "favorite color ".to_string()),
        ];

        let (important, other) = partition_client_fields(&client, &configured);

        assert_eq!(
            important,
            vec![
                ClientFieldDisplay {
                    label: "Stage".to_string(),
                    value: Some("In progress".to_string()),
                },
                ClientFieldDisplay {
                    label: "Missing".to_string(),
                    value: None,
                },
                ClientFieldDisplay {
                    label: "favorite color".to_string(),
                    value: Some("Blue".to_string()),
                },
            ]
        );

        assert_eq!(
            other,
            vec![ClientFieldDisplay {
                label: "Other".to_string(),
                value: None,
            }]
        );
    }

    #[test]
    fn partition_client_fields_handles_missing_custom_fields() {
        let client = client_with_fields(vec![]);
        let configured = vec![ImportantField::new(1, "Stage".to_string())];

        let (important, other) = partition_client_fields(&client, &configured);

        assert_eq!(
            important,
            vec![ClientFieldDisplay {
                label: "Stage".to_string(),
                value: None,
            }]
        );
        assert!(other.is_empty());
    }
}

/// Generic result wrapper for client mutations so callers can redirect easily.
#[derive(Debug)]
pub struct ClientOperationOutcome {
    pub client_id: i32,
}

/// Ensures that the current user has access to the provided client identifier.
fn ensure_client_access<R>(repo: &R, user: &AuthenticatedUser, client_id: i32) -> ServiceResult<()>
where
    R: ClientReader + ?Sized,
{
    if check_role("crm_manager", &user.roles) {
        match is_client_assigned_to_manager(repo, client_id, &user.email) {
            Ok(true) => Ok(()),
            Ok(false) => {
                log::warn!(
                    "Manager {email} attempted to access forbidden client {client_id}",
                    email = user.email
                );
                Err(ServiceError::Unauthorized)
            }
            Err(err) => {
                log::error!(
                    "Failed to verify access for manager {email} to client {client_id}: {err}",
                    email = user.email
                );
                Err(err)
            }
        }
    } else {
        Ok(())
    }
}

/// Returns [`ServiceError::Unauthorized`] when the user lacks the CRM role.
fn ensure_service_access(user: &AuthenticatedUser) -> ServiceResult<()> {
    if check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        Ok(())
    } else {
        log::warn!(
            "User {email} does not have required role {role}",
            email = user.email,
            role = SERVICE_ACCESS_ROLE
        );
        Err(ServiceError::Unauthorized)
    }
}

/// Loads a client by id or returns [`ServiceError::NotFound`] with logging.
fn load_client_or_not_found<R>(repo: &R, hub_id: i32, client_id: i32) -> ServiceResult<Client>
where
    R: ClientReader + ?Sized,
{
    match get_client_by_id(repo, client_id, hub_id).map_err(|err| {
        log::error!("Failed to fetch client {client_id}: {err}");
        err
    })? {
        Some(client) => Ok(client),
        None => {
            log::warn!(
                "Client {client_id} not found for hub {hub_id}",
                hub_id = hub_id
            );
            Err(ServiceError::NotFound)
        }
    }
}

/// Aggregates all data required by the client details page, applying access rules.
pub fn load_client_details<R>(
    repo: &R,
    user: &AuthenticatedUser,
    client_id: i32,
) -> ServiceResult<ClientPageData>
where
    R: ClientReader + ClientEventReader + ImportantFieldReader + ?Sized,
{
    ensure_service_access(user)?;
    ensure_client_access(repo, user, client_id)?;

    let client = load_client_or_not_found(repo, user.hub_id, client_id)?;

    let managers = list_client_managers(repo, client.id).map_err(|err| {
        log::error!("Failed to load managers for client {client_id}: {err}");
        err
    })?;

    let (total_events, events_with_managers) =
        list_client_events(repo, ClientEventListQuery::new(client.id)).map_err(|err| {
            log::error!("Failed to load events for client {client_id}: {err}");
            err
        })?;

    let documents = events_with_managers
        .iter()
        .filter(|&(event, _)| event.event_type == ClientEventType::DocumentLink)
        .map(|(event, _)| event.clone())
        .collect::<Vec<_>>();

    let available_fields = list_available_fields(repo, user.hub_id).map_err(|err| {
        log::error!(
            "Failed to load available fields for hub {hub_id}: {err}",
            hub_id = user.hub_id
        );
        err
    })?;

    let important_field_names = repo.list_important_fields(user.hub_id).map_err(|err| {
        log::error!(
            "Failed to load important fields for hub {hub_id}: {err}",
            hub_id = user.hub_id
        );
        err
    })?;

    let (important_fields, other_fields) = partition_client_fields(&client, &important_field_names);

    Ok(ClientPageData {
        client,
        managers,
        events_with_managers,
        documents,
        available_fields,
        important_fields,
        other_fields,
        total_events,
    })
}

/// Applies updates submitted through the save client form.
pub fn save_client<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: SaveClientForm,
) -> ServiceResult<ClientOperationOutcome>
where
    R: ClientReader + ClientWriter + ?Sized,
{
    ensure_service_access(user)?;

    if let Err(err) = form.validate() {
        log::error!("Failed to validate save client form: {err}");
        return Err(ServiceError::Form("Ошибка валидации формы".to_string()));
    }

    let client_id = form.id;
    let updates: UpdateClient = form.into();

    let client = load_client_or_not_found(repo, user.hub_id, client_id)?;

    ensure_client_access(repo, user, client.id)?;

    let updated_client = update_client(repo, client.id, &updates).map_err(|err| {
        log::error!("Failed to update client {client_id}: {err}");
        err
    })?;

    Ok(ClientOperationOutcome {
        client_id: updated_client.id,
    })
}

/// Adds a comment or event for a client, sending emails when requested.
pub async fn add_comment<R>(
    repo: &R,
    user: &AuthenticatedUser,
    zmq_sender: &ZmqSender,
    form: AddCommentForm,
) -> ServiceResult<ClientOperationOutcome>
where
    R: ClientReader + ClientEventWriter + ManagerWriter + ?Sized,
{
    ensure_service_access(user)?;

    if let Err(err) = form.validate() {
        log::error!("Failed to validate comment form: {err}");
        return Err(ServiceError::Form("Ошибка валидации формы".to_string()));
    }

    let client_id = form.id;
    let event_type = ClientEventType::from(form.event_type.as_str());
    let subject = form.subject.clone();
    let sanitized_message = ammonia::clean(&form.message);

    ensure_client_access(repo, user, client_id)?;

    let manager = create_or_update_manager(repo, &user.into()).map_err(|err| {
        log::error!(
            "Failed to create or update manager {email}: {err}",
            email = user.email
        );
        err
    })?;

    let client = load_client_or_not_found(repo, user.hub_id, client_id)?;

    if matches!(event_type, ClientEventType::Email) {
        let client_email = client.email.as_ref().ok_or_else(|| {
            log::warn!("Client {client_id} has no email to send message");
            ServiceError::Form("Клиент не имеет email".to_string())
        })?;

        let fields: HashMap<String, String> = client
            .fields
            .clone()
            .map(|map| map.into_iter().collect())
            .unwrap_or_default();

        let new_email = NewEmail {
            message: sanitized_message.clone(),
            subject: subject.clone(),
            attachment: None,
            attachment_name: None,
            attachment_mime: None,
            hub_id: user.hub_id,
            recipients: vec![NewEmailRecipient {
                address: client_email.clone(),
                name: client.name.clone(),
                fields,
            }],
        };

        let zmq_message = ZMQSendEmailMessage::NewEmail(Box::new((user.clone(), new_email)));

        if let Err(err) = zmq_sender.send_json(&zmq_message).await {
            log::error!("Failed to enqueue email for client {client_id}: {err}");
            return Err(ServiceError::Internal);
        }
    }

    let mut event_data = json!({ "text": sanitized_message });
    if let Some(subject) = subject {
        event_data["subject"] = json!(subject);
    }

    let new_event = NewClientEvent {
        client_id: client.id,
        event_type,
        manager_id: manager.id,
        created_at: Utc::now().naive_utc(),
        event_data,
    };

    create_client_event(repo, &new_event).map_err(|err| {
        log::error!("Failed to create event for client {client_id}: {err}");
        err
    })?;

    Ok(ClientOperationOutcome {
        client_id: client.id,
    })
}

/// Adds an attachment event for the client.
pub fn add_attachment<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: AddAttachmentForm,
) -> ServiceResult<ClientOperationOutcome>
where
    R: ClientReader + ClientEventWriter + ManagerWriter + ?Sized,
{
    ensure_service_access(user)?;

    if let Err(err) = form.validate() {
        log::error!("Failed to validate attachment form: {err}");
        return Err(ServiceError::Form("Ошибка валидации формы".to_string()));
    }

    let client_id = form.id;

    ensure_client_access(repo, user, client_id)?;

    let manager = create_or_update_manager(repo, &user.into()).map_err(|err| {
        log::error!(
            "Failed to create or update manager {email}: {err}",
            email = user.email
        );
        err
    })?;

    let client = load_client_or_not_found(repo, user.hub_id, client_id)?;

    let event = NewClientEvent {
        client_id: client.id,
        event_type: ClientEventType::DocumentLink,
        manager_id: manager.id,
        created_at: Utc::now().naive_utc(),
        event_data: json!({
            "text": form.text,
            "url": form.url,
        }),
    };

    create_client_event(repo, &event).map_err(|err| {
        log::error!("Failed to create attachment for client {client_id}: {err}");
        err
    })?;

    Ok(ClientOperationOutcome {
        client_id: client.id,
    })
}

/// Fetches a client by its identifier scoped to the provided hub.
pub fn get_client_by_id<R>(repo: &R, client_id: i32, hub_id: i32) -> ServiceResult<Option<Client>>
where
    R: ClientReader + ?Sized,
{
    repo.get_client_by_id(client_id, hub_id)
        .map_err(ServiceError::from)
}

/// Returns the managers linked to the given client.
pub fn list_client_managers<R>(repo: &R, client_id: i32) -> ServiceResult<Vec<Manager>>
where
    R: ClientReader + ?Sized,
{
    repo.list_managers(client_id).map_err(ServiceError::from)
}

/// Returns the available client fields for a hub
pub fn list_available_fields<R>(repo: &R, hub_id: i32) -> ServiceResult<Vec<String>>
where
    R: ClientReader + ?Sized,
{
    repo.list_available_fields(hub_id)
        .map_err(ServiceError::from)
}

/// Retrieves the paginated list of client events with their managers.
pub fn list_client_events<R>(
    repo: &R,
    query: ClientEventListQuery,
) -> ServiceResult<(usize, Vec<(ClientEvent, Manager)>)>
where
    R: ClientEventReader + ?Sized,
{
    repo.list_client_events(query).map_err(ServiceError::from)
}

/// Checks whether the client is assigned to the specified manager email.
pub fn is_client_assigned_to_manager<R>(
    repo: &R,
    client_id: i32,
    manager_email: &str,
) -> ServiceResult<bool>
where
    R: ClientReader + ?Sized,
{
    repo.check_client_assigned_to_manager(client_id, manager_email)
        .map_err(ServiceError::from)
}

/// Applies the provided updates to the client entity.
pub fn update_client<R>(repo: &R, client_id: i32, updates: &UpdateClient) -> ServiceResult<Client>
where
    R: ClientWriter + ?Sized,
{
    repo.update_client(client_id, updates)
        .map_err(ServiceError::from)
}

/// Persists or updates the manager derived from the provided data.
pub fn create_or_update_manager<R>(repo: &R, new_manager: &NewManager) -> ServiceResult<Manager>
where
    R: ManagerWriter + ?Sized,
{
    repo.create_or_update_manager(new_manager)
        .map_err(ServiceError::from)
}

/// Persists a new client event.
pub fn create_client_event<R>(repo: &R, event: &NewClientEvent) -> ServiceResult<ClientEvent>
where
    R: ClientEventWriter + ?Sized,
{
    repo.create_client_event(event).map_err(ServiceError::from)
}

/// Lists all managers for the provided hub with their assigned clients.
pub fn list_managers_with_clients<R>(
    repo: &R,
    hub_id: i32,
) -> ServiceResult<Vec<(Manager, Vec<Client>)>>
where
    R: ManagerReader + ?Sized,
{
    repo.list_managers_with_clients(hub_id)
        .map_err(ServiceError::from)
}

/// Creates a batch of clients returning the count of inserted rows.
pub fn create_clients<R>(repo: &R, new_clients: &[NewClient]) -> ServiceResult<usize>
where
    R: ClientWriter + ?Sized,
{
    repo.create_clients(new_clients).map_err(ServiceError::from)
}

/// Assigns the provided list of client identifiers to the given manager.
pub fn assign_clients_to_manager<R>(
    repo: &R,
    manager_id: i32,
    client_ids: &[i32],
) -> ServiceResult<usize>
where
    R: ManagerWriter + ?Sized,
{
    repo.assign_clients_to_manager(manager_id, client_ids)
        .map_err(ServiceError::from)
}
