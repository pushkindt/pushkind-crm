use std::io::Read;

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use csv;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use crate::domain::client::NewClient;

#[derive(Deserialize, Validate)]
pub struct AddClientForm {
    pub hub_id: i32,
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    pub phone: String,
    pub address: String,
}

impl From<AddClientForm> for NewClient {
    fn from(form: AddClientForm) -> Self {
        Self {
            hub_id: form.hub_id,
            name: form.name,
            email: form.email,
            phone: form.phone,
            address: form.address,
        }
    }
}

#[derive(MultipartForm)]
pub struct UploadClientsForm {
    #[multipart(limit = "10MB")]
    pub csv: TempFile,
}

#[derive(Debug, Error)]
pub enum UploadClientsFormError {
    #[error("Error reading csv file")]
    FileReadError,
    #[error("Error parsing csv file")]
    CsvParseError,
}

impl From<std::io::Error> for UploadClientsFormError {
    fn from(_: std::io::Error) -> Self {
        UploadClientsFormError::FileReadError
    }
}

impl From<csv::Error> for UploadClientsFormError {
    fn from(_: csv::Error) -> Self {
        UploadClientsFormError::CsvParseError
    }
}

#[derive(Debug, Deserialize)]
struct CsvClientRow {
    name: String,
    email: String,
    phone: String,
    address: String,
}

impl UploadClientsForm {
    pub fn parse(&mut self, hub_id: i32) -> Result<Vec<NewClient>, UploadClientsFormError> {
        let mut csv_content = String::new();
        self.csv.file.read_to_string(&mut csv_content)?;

        let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());

        let mut clients = Vec::new();

        for result in rdr.deserialize::<CsvClientRow>() {
            let row = result?;

            clients.push(NewClient {
                hub_id,
                name: row.name,
                email: row.email,
                phone: row.phone,
                address: row.address,
            });
        }

        Ok(clients)
    }
}
