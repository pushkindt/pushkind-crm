use std::{collections::HashMap, io::Read};

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
        NewClient::new(
            form.hub_id,
            form.name,
            form.email,
            form.phone,
            form.address,
            None,
        )
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

impl UploadClientsForm {
    /// Parse the uploaded CSV file into a list of [`NewClient`] records.
    pub fn parse(&mut self, hub_id: i32) -> Result<Vec<NewClient>, UploadClientsFormError> {
        let mut csv_content = String::new();
        self.csv.file.read_to_string(&mut csv_content)?;

        let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());

        let mut clients = Vec::new();

        let headers = rdr.headers()?.clone();

        for result in rdr.records() {
            let record = result?;
            let mut optional_fields = HashMap::new();

            let mut name = String::new();
            let mut email = String::new();
            let mut phone = String::new();
            let mut address = String::new();

            for (i, field) in record.iter().enumerate() {
                match headers.get(i) {
                    Some("name") => name = field.trim().to_string(),
                    Some("email") => email = field.trim().to_string(),
                    Some("phone") => phone = field.trim().to_string(),
                    Some("address") => address = field.trim().to_string(),
                    Some(header) => {
                        if field.is_empty() {
                            continue;
                        }
                        optional_fields.insert(header.to_string(), field.to_string());
                    }
                    None => continue,
                }
            }

            if name.is_empty() || email.is_empty() || phone.is_empty() || address.is_empty() {
                // Skip records missing required fields.
                continue;
            }

            clients.push(NewClient::new(
                hub_id,
                name,
                email,
                phone,
                address,
                Some(optional_fields),
            ));
        }

        Ok(clients)
    }
}
