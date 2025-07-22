use serde::Deserialize;
use validator::Validate;

use crate::domain::client::UpdateClient;

#[derive(Deserialize, Validate)]
pub struct SaveClientForm {
    pub id: i32,
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    pub phone: String,
    pub address: String,
}

#[derive(Deserialize, Validate)]
pub struct AddCommentForm {
    pub id: i32,
    #[validate(length(min = 1))]
    pub text: String,
    pub event_type: String,
}

#[derive(Deserialize, Validate)]
pub struct AddAttachmentForm {
    pub id: i32,
    #[validate(length(min = 1))]
    pub text: String,
    #[validate(url)]
    pub url: String,
}

impl<'a> From<&'a SaveClientForm> for UpdateClient<'a> {
    fn from(form: &'a SaveClientForm) -> Self {
        Self {
            name: &form.name,
            email: &form.email,
            phone: &form.phone,
            address: &form.address,
        }
    }
}
