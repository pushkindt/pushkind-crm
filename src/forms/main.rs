use serde::Deserialize;

use crate::domain::client::NewClient;

#[derive(Deserialize)]
pub struct AddClientForm {
    pub hub_id: i32,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
}

impl<'a> From<&'a AddClientForm> for NewClient<'a> {
    fn from(form: &'a AddClientForm) -> Self {
        Self {
            hub_id: form.hub_id,
            name: form.name.as_str(),
            email: form.email.as_str(),
            phone: form.phone.as_str(),
            address: form.address.as_str(),
        }
    }
}
