use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct AddManagerForm {
    pub id: i32,
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(email)]
    pub email: String,
}

#[derive(Deserialize)]
pub struct AssignManagerForm {
    pub manager_id: i32,
    #[serde(default)]
    pub client_ids: Vec<i32>,
}
