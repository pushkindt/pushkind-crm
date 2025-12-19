//! Forms that validate and normalize client input.

use std::collections::BTreeMap;

use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use validator::Validate;

use crate::domain::client::UpdateClient;
use crate::domain::client_event::ClientEventType;
use crate::domain::types::{
    AttachmentName, AttachmentUrl, ClientEmail, ClientName, CommentMessage, CommentSubject,
    PhoneNumber,
};
use crate::forms::FormError;

#[derive(Deserialize, Validate)]
/// Form data for updating an existing client.
pub struct SaveClientForm {
    /// Updated display name.
    #[validate(length(min = 1))]
    pub name: String,
    /// Updated email.
    #[validate(email)]
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub email: Option<String>,
    /// Updated contact phone number.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub phone: Option<String>,
    #[serde(default)]
    pub field: Vec<String>,
    #[serde(default)]
    pub value: Vec<String>,
}

pub struct SaveClientPayload {
    pub name: ClientName,
    pub email: Option<ClientEmail>,
    pub phone: Option<PhoneNumber>,
    pub fields: Option<BTreeMap<String, String>>,
}

#[derive(Deserialize, Validate)]
/// Form data for adding a comment to a client.
pub struct AddCommentForm {
    /// Optional subject if the comment is an email.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub subject: Option<String>,
    /// Comment text content.
    #[validate(length(min = 1))]
    pub message: String,
    /// Type of event associated with the comment.
    #[validate(length(min = 1))]
    pub event_type: String,
}

pub struct AddCommentPayload {
    pub subject: Option<CommentSubject>,
    pub message: CommentMessage,
    pub event_type: ClientEventType,
}

#[derive(Deserialize, Validate)]
/// Form data for adding an attachment to a client.
pub struct AddAttachmentForm {
    /// Attachment description.
    #[validate(length(min = 1))]
    pub text: String,
    /// URL pointing to the attachment.
    #[validate(url)]
    pub url: String,
}

pub struct AddAttachmentPayload {
    pub text: AttachmentName,
    pub url: AttachmentUrl,
}

impl TryFrom<SaveClientForm> for SaveClientPayload {
    type Error = FormError;

    /// Convert the [`SaveClientForm`] into an [`SaveClientPayload`] value for persistence.
    fn try_from(form: SaveClientForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        let fields: BTreeMap<String, String> = form
            .field
            .iter()
            .zip(form.value.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let name = ClientName::new(form.name).map_err(|_| FormError::InvalidName)?;
        let email = form
            .email
            .map(ClientEmail::try_from)
            .transpose()
            .map_err(|_| FormError::InvalidEmail)?;
        let phone = match form.phone {
            Some(value) => {
                Some(PhoneNumber::try_from(value).map_err(|_| FormError::InvalidPhoneNumber)?)
            }
            None => None,
        };

        Ok(SaveClientPayload {
            name,
            email,
            phone,
            fields: Some(fields),
        })
    }
}

impl From<SaveClientPayload> for UpdateClient {
    /// Convert the [`SaveClientPayload`] into an [`UpdateClient`] value for persistence.
    fn from(payload: SaveClientPayload) -> Self {
        UpdateClient::new(payload.name, payload.email, payload.phone, payload.fields)
    }
}

impl TryFrom<AddCommentForm> for AddCommentPayload {
    type Error = FormError;

    /// Convert the [`AddCommentForm`] into an [`AddCommentPayload`] value for persistence.
    fn try_from(form: AddCommentForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        let subject = match form.subject {
            Some(value) => Some(CommentSubject::new(value).map_err(|_| FormError::InvalidName)?),
            None => None,
        };

        let message = CommentMessage::new(form.message).map_err(|_| FormError::InvalidName)?;

        let event_type = ClientEventType::from(form.event_type.as_str());

        Ok(AddCommentPayload {
            subject,
            message,
            event_type,
        })
    }
}

impl TryFrom<AddAttachmentForm> for AddAttachmentPayload {
    type Error = FormError;

    /// Convert the [`AddAttachmentForm`] into an [`AddAttachmentPayload`] value for persistence.
    fn try_from(form: AddAttachmentForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        let text = AttachmentName::new(form.text).map_err(|_| FormError::InvalidName)?;
        let url = AttachmentUrl::new(form.url).map_err(|_| FormError::InvalidUrl)?;

        Ok(AddAttachmentPayload { text, url })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_client_form_into_update_client_normalizes_optional_fields() {
        let form = SaveClientForm {
            name: "Alice".to_string(),
            email: Some("Alice@Example.COM".to_string()),
            phone: Some("+1 (415) 555-2671".to_string()),
            field: vec!["tier".to_string()],
            value: vec!["gold".to_string()],
        };

        let payload = SaveClientPayload::try_from(form).expect("expected normalized update client");

        let update = UpdateClient::from(payload);

        assert_eq!(
            update.email.as_ref().map(|email| email.as_str()),
            Some("alice@example.com")
        );
        assert_eq!(
            update.phone.as_ref().map(|phone| phone.as_str()),
            Some("+14155552671")
        );

        let fields = update.fields.expect("fields should be populated");
        assert_eq!(fields.get("tier"), Some(&"gold".to_string()));
    }

    #[test]
    fn save_client_form_into_update_client_drops_empty_fields() {
        let form = SaveClientForm {
            name: "Bob".to_string(),
            email: None,
            phone: None,
            field: Vec::new(),
            value: Vec::new(),
        };

        let payload = SaveClientPayload::try_from(form).expect("expected normalized update client");

        let update = UpdateClient::from(payload);

        assert!(update.fields.is_none());
    }

    #[test]
    fn add_comment_form_into_payload_sanitizes_and_parses() {
        let form = AddCommentForm {
            subject: Some("Follow up".to_string()),
            message: "<b>Hello</b>".to_string(),
            event_type: "email".to_string(),
        };

        let payload = AddCommentPayload::try_from(form).expect("expected comment payload");

        assert_eq!(
            payload.subject.as_ref().map(CommentSubject::as_str),
            Some("Follow up")
        );
        assert_eq!(payload.message.as_str(), "<b>Hello</b>");
        assert_eq!(payload.event_type, ClientEventType::Email);
    }

    #[test]
    fn add_attachment_form_into_payload_validates_fields() {
        let form = AddAttachmentForm {
            text: "Document".to_string(),
            url: "https://example.com/doc.pdf".to_string(),
        };

        let payload = AddAttachmentPayload::try_from(form).expect("expected attachment payload");

        assert_eq!(payload.text.as_str(), "Document");
        assert_eq!(payload.url.as_str(), "https://example.com/doc.pdf");
    }
}
