//! Domain services orchestrating client operations.

use std::collections::BTreeMap;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::check_role;
use pushkind_common::routes::ensure_role;
use pushkind_common::zmq::ZmqSender;
use pushkind_common::zmq::ZmqSenderExt;
use pushkind_emailer::domain::email::{NewEmail, NewEmailRecipient};
use pushkind_emailer::domain::types::EmailBody;
use pushkind_emailer::domain::types::EmailSubject;
use pushkind_emailer::domain::types::HubId as EmailerHubId;
use pushkind_emailer::domain::types::RecipientEmail;
use pushkind_emailer::domain::types::RecipientName;
use pushkind_emailer::models::zmq::ZMQSendEmailMessage;
use serde_json::json;

use crate::SERVICE_ACCESS_ROLE;
use crate::SERVICE_MANAGER_ROLE;
use crate::domain::client::{Client, UpdateClient};
use crate::domain::client_event::{ClientEventType, NewClientEvent};
use crate::domain::important_field::ImportantField;
use crate::domain::manager::NewManager;
use crate::domain::types::ClientId;
use crate::domain::types::HubId;
use crate::domain::types::ManagerEmail;
use crate::dto::client::{ClientFieldDisplay, ClientOperationOutcome, ClientPageData};
use crate::forms::client::AddAttachmentPayload;
use crate::forms::client::AddCommentPayload;
use crate::forms::client::SaveClientPayload;
use crate::forms::client::{AddAttachmentForm, AddCommentForm, SaveClientForm};
use crate::repository::{
    ClientEventListQuery, ClientEventReader, ClientEventWriter, ClientReader, ClientWriter,
    ImportantFieldReader, ManagerWriter,
};
use crate::services::{ServiceError, ServiceResult};

/// Splits client fields into configured important labels and the remaining entries.
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
        let label = field.field.as_str().trim();
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

/// Trims and normalizes a field value, returning `None` when empty.
fn normalize_field_value(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Ensures that the current user has access to the provided client identifier.
fn ensure_client_access<R>(
    client_id: ClientId,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: ClientReader + ?Sized,
{
    if !check_role(SERVICE_MANAGER_ROLE, &user.roles) {
        return Ok(());
    }

    repo.check_client_assigned_to_manager(client_id, &ManagerEmail::new(&user.email)?)?
        .then_some(())
        .ok_or(ServiceError::Unauthorized)
}

/// Aggregates all data required by the client details page, applying access rules.
pub fn load_client_details<R>(
    client_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ClientPageData>
where
    R: ClientReader + ClientEventReader + ImportantFieldReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let client_id = ClientId::new(client_id)?;
    let hub_id = HubId::new(user.hub_id)?;

    ensure_client_access(client_id, user, repo)?;

    let client = repo
        .get_client_by_id(client_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let managers = repo.list_managers(client_id)?;

    let (total_events, events_with_managers) =
        repo.list_client_events(ClientEventListQuery::new(client_id))?;

    let documents = events_with_managers
        .iter()
        .filter(|&(event, _)| event.event_type == ClientEventType::DocumentLink)
        .map(|(event, _)| event.clone())
        .collect::<Vec<_>>();

    let available_fields = repo.list_available_fields(hub_id)?;

    let important_field_names = repo.list_important_fields(hub_id)?;

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
    client_id: i32,
    form: SaveClientForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ClientOperationOutcome>
where
    R: ClientReader + ClientWriter + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let client_id = ClientId::new(client_id)?;
    let hub_id = HubId::new(user.hub_id)?;

    let payload = SaveClientPayload::try_from(form)?;

    let updates: UpdateClient = payload.into();

    let client = repo
        .get_client_by_id(client_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    ensure_client_access(client.id, user, repo)?;

    let updated_client = repo.update_client(client_id, &updates)?;

    Ok(ClientOperationOutcome {
        client_id: updated_client.id,
    })
}

/// Adds a comment or event for a client, sending emails when requested.
pub async fn add_comment<R>(
    client_id: i32,
    form: AddCommentForm,
    user: &AuthenticatedUser,
    repo: &R,
    zmq_sender: &ZmqSender,
) -> ServiceResult<ClientOperationOutcome>
where
    R: ClientReader + ClientEventWriter + ManagerWriter + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let client_id = ClientId::new(client_id)?;
    let hub_id = HubId::new(user.hub_id)?;

    ensure_client_access(client_id, user, repo)?;

    let payload = AddCommentPayload::try_from(form)?;

    let manager_payload = NewManager::try_from(user).map_err(|err| {
        log::error!("Failed to build manager from user: {err}");
        ServiceError::Internal
    })?;

    let manager = repo.create_or_update_manager(&manager_payload)?;

    let client = repo
        .get_client_by_id(client_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    if matches!(payload.event_type, ClientEventType::Email) {
        let client_email = client.email.as_ref().ok_or_else(|| {
            log::warn!("Client {client_id} has no email to send message");
            ServiceError::Form("Клиент не имеет email".to_string())
        })?;

        let fields: BTreeMap<String, String> = client.fields.clone().unwrap_or_default();

        let hub_id = EmailerHubId::new(user.hub_id)?;

        let new_email = NewEmail {
            message: EmailBody::new(payload.message.as_str())?,
            subject: payload
                .subject
                .as_deref()
                .map(EmailSubject::new)
                .transpose()?,
            attachment: None,
            attachment_name: None,
            attachment_mime: None,
            hub_id,
            recipients: vec![NewEmailRecipient {
                address: RecipientEmail::new(client_email.clone().into_inner())?,
                name: RecipientName::new(client.name.as_str())?,
                fields,
            }],
        };

        let zmq_message = ZMQSendEmailMessage::NewEmail(Box::new((user.clone(), new_email)));

        if let Err(err) = zmq_sender.send_json(&zmq_message).await {
            log::error!("Failed to enqueue email for client {client_id}: {err}");
            return Err(ServiceError::Internal);
        }
    }

    let mut event_data = json!({ "text": payload.message.as_str() });
    if let Some(subject) = payload.subject {
        event_data["subject"] = json!(subject.as_str());
    }

    let new_event = NewClientEvent::new(client.id, manager.id, payload.event_type, event_data);

    repo.create_client_event(&new_event)?;

    Ok(ClientOperationOutcome {
        client_id: client.id,
    })
}

/// Adds an attachment event for the client.
pub fn add_attachment<R>(
    client_id: i32,
    form: AddAttachmentForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ClientOperationOutcome>
where
    R: ClientReader + ClientEventWriter + ManagerWriter + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let client_id = ClientId::new(client_id)?;
    let hub_id = HubId::new(user.hub_id)?;

    ensure_client_access(client_id, user, repo)?;

    let payload = AddAttachmentPayload::try_from(form)?;

    let manager_payload = NewManager::try_from(user).map_err(|err| {
        log::error!("Failed to build manager from user: {err}");
        ServiceError::Internal
    })?;
    let manager = repo.create_or_update_manager(&manager_payload)?;

    let client = repo
        .get_client_by_id(client_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let event = NewClientEvent::new(
        client.id,
        manager.id,
        ClientEventType::DocumentLink,
        json!({
            "text": payload.text.as_str(),
            "url": payload.url.as_str(),
        }),
    );

    repo.create_client_event(&event)?;

    Ok(ClientOperationOutcome {
        client_id: client.id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{ClientId, ClientName, HubId};
    use chrono::Utc;
    use std::collections::BTreeMap;

    /// Creates a test client populated with the given field pairs.
    fn client_with_fields(fields: Vec<(&str, &str)>) -> Client {
        let mut map = BTreeMap::new();
        for (key, value) in fields {
            map.insert(key.to_string(), value.to_string());
        }

        Client {
            id: ClientId::new(1).expect("valid client id"),
            hub_id: HubId::new(1).expect("valid hub id"),
            name: ClientName::new("Test").expect("valid name"),
            email: None,
            phone: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            fields: if map.is_empty() { None } else { Some(map) },
        }
    }

    fn configured_field(hub: i32, name: &str) -> ImportantField {
        ImportantField::try_new(hub, name.to_string()).expect("valid important field")
    }

    /// Verifies that configured names are extracted and normalized correctly.
    #[test]
    fn partition_client_fields_extracts_configured_names() {
        let client = client_with_fields(vec![
            ("Favorite Color", "  Blue  "),
            (" Stage ", " In progress "),
            ("Other", "  "),
        ]);

        let configured = vec![
            configured_field(1, "Stage"),
            configured_field(1, "Missing"),
            configured_field(1, "favorite color "),
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

    /// Verifies that missing configured fields yield empty values.
    #[test]
    fn partition_client_fields_handles_missing_custom_fields() {
        let client = client_with_fields(vec![]);
        let configured = vec![configured_field(1, "Stage")];

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
