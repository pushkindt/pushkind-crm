use std::collections::BTreeMap;

use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use validator::Validate;

use crate::domain::client::UpdateClient;

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
    /// Updated address.
    #[serde(deserialize_with = "empty_string_as_none")]
    pub address: Option<String>,
    /// Updated contact person.
    #[serde(deserialize_with = "empty_string_as_none")]
    pub contact: Option<String>,
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

impl From<SaveClientForm> for UpdateClient {
    /// Convert the [`SaveClientForm`] into an [`UpdateClient`] value for persistence.
    fn from(form: SaveClientForm) -> Self {
        let fields: BTreeMap<String, String> = form
            .field
            .iter()
            .zip(form.value.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        UpdateClient::new(
            form.name,
            form.email,
            form.phone,
            form.address,
            form.contact,
            Some(fields),
        )
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
            address: Some(" 1 Market St ".to_string()),
            contact: Some("  Bob  ".to_string()),
            field: vec!["tier".to_string()],
            value: vec!["gold".to_string()],
        };

        let update: UpdateClient = form.into();

        assert_eq!(update.email.as_deref(), Some("alice@example.com"));
        assert_eq!(update.phone.as_deref(), Some("+14155552671"));
        assert_eq!(update.address.as_deref(), Some("1 Market St"));
        assert_eq!(update.contact.as_deref(), Some("Bob"));

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
            address: None,
            contact: None,
            field: Vec::new(),
            value: Vec::new(),
        };

        let update: UpdateClient = form.into();

        assert!(update.fields.is_none());
    }
}
