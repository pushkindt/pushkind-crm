//! Forms that validate and normalize client input.

use std::collections::BTreeMap;

use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use validator::Validate;

use crate::domain::client::UpdateClient;
use crate::domain::types::{ClientEmail, ClientName, PhoneNumber, TypeConstraintError};

#[derive(Deserialize, Validate)]
/// Form data for updating an existing client.
pub struct SaveClientForm {
    /// Client identifier.
    pub id: i32,
    /// Updated display name.
    #[validate(length(min = 1))]
    pub name: String,
    /// Updated email.
    #[validate(email)]
    #[serde(deserialize_with = "empty_string_as_none")]
    pub email: Option<String>,
    /// Updated contact phone number.
    #[serde(deserialize_with = "empty_string_as_none")]
    pub phone: Option<String>,
    #[serde(default)]
    pub field: Vec<String>,
    #[serde(default)]
    pub value: Vec<String>,
}

#[derive(Deserialize, Validate)]
/// Form data for adding a comment to a client.
pub struct AddCommentForm {
    /// Identifier of the client that receives the comment.
    pub id: i32,
    /// Optional subject if the comment is an email.
    pub subject: Option<String>,
    /// Comment text content.
    #[validate(length(min = 1))]
    pub message: String,
    /// Type of event associated with the comment.
    pub event_type: String,
}

#[derive(Deserialize, Validate)]
/// Form data for adding an attachment to a client.
pub struct AddAttachmentForm {
    /// Identifier of the client that receives the attachment.
    pub id: i32,
    /// Attachment description.
    #[validate(length(min = 1))]
    pub text: String,
    /// URL pointing to the attachment.
    #[validate(url)]
    pub url: String,
}

impl TryFrom<SaveClientForm> for UpdateClient {
    type Error = TypeConstraintError;

    /// Convert the [`SaveClientForm`] into an [`UpdateClient`] value for persistence.
    fn try_from(form: SaveClientForm) -> Result<Self, Self::Error> {
        let fields: BTreeMap<String, String> = form
            .field
            .iter()
            .zip(form.value.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let name = ClientName::new(form.name)?;
        let email = form.email.map(ClientEmail::try_from).transpose()?;
        let phone = form
            .phone
            .and_then(|value| PhoneNumber::try_from(value).ok());

        Ok(UpdateClient::new(name, email, phone, Some(fields)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_client_form_into_update_client_normalizes_optional_fields() {
        let form = SaveClientForm {
            id: 1,
            name: "Alice".to_string(),
            email: Some("Alice@Example.COM".to_string()),
            phone: Some("+1 (415) 555-2671".to_string()),
            field: vec!["tier".to_string()],
            value: vec!["gold".to_string()],
        };

        let update = UpdateClient::try_from(form).expect("expected normalized update client");

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
            id: 2,
            name: "Bob".to_string(),
            email: None,
            phone: None,
            field: Vec::new(),
            value: Vec::new(),
        };

        let update = UpdateClient::try_from(form).expect("expected normalized update client");

        assert!(update.fields.is_none());
    }
}
