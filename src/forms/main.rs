use std::io::Read;

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use crate::domain::client::NewClient;

#[derive(Deserialize, Validate)]
/// Form data used to add a new client.
pub struct AddClientForm {
    /// Identifier of the hub that owns the client.
    pub hub_id: i32,
    /// Client's display name.
    #[validate(length(min = 1))]
    pub name: String,
    /// Client's email address.
    #[validate(email)]
    pub email: String,
    /// Contact phone number.
    pub phone: String,
    /// Mailing address.
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
/// Multipart form for uploading a CSV file with new clients.
pub struct UploadClientsForm {
    #[multipart(limit = "10MB")]
    /// Uploaded CSV file containing client data.
    pub csv: TempFile,
}

#[derive(Debug, Error)]
/// Errors that can occur while parsing an uploaded clients CSV file.
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
/// Representation of a client row in the uploaded CSV file.
struct CsvClientRow {
    name: String,
    email: String,
    phone: String,
    address: String,
}

impl UploadClientsForm {
    /// Parse the uploaded CSV file into a list of [`NewClient`] records.
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
