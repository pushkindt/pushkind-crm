use serde::Deserialize;


#[derive(Deserialize)]
pub struct AddManagerForm {
    pub id: i32,
    pub name: String,
    pub email: String,
}
