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
    /// Comment text content.
    #[validate(length(min = 1))]
    pub text: String,
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

        UpdateClient::new(form.name, form.email, form.phone, Some(fields))
    }
}
