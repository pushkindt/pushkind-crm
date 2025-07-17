use serde::Deserialize;

use crate::domain::client::UpdateClient;

#[derive(Deserialize)]
pub struct SaveClientForm {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
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
