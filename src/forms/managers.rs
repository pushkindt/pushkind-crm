use serde::Deserialize;

#[derive(Deserialize)]
pub struct AddManagerForm {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct AssignClientsForm {
    pub manager_id: i32,
    #[serde(default)]
    pub client_ids: Vec<i32>,
}
