//! Forms tied to the main CRM dashboard.

use std::{collections::BTreeMap, io::Read};

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use crate::domain::client::NewClient;

#[derive(Deserialize, Validate)]
/// Form data used to add a new client.
pub struct AddClientForm {
    /// Client's display name.
    #[validate(length(min = 1))]
    pub name: String,
    /// Client's email.
    #[validate(email)]
    #[serde(deserialize_with = "empty_string_as_none")]
    pub email: Option<String>,
    /// Contact phone number.
    #[serde(deserialize_with = "empty_string_as_none")]
    pub phone: Option<String>,
}

impl AddClientForm {
    pub fn to_new_client(self, hub_id: i32) -> NewClient {
        NewClient::new(hub_id, self.name, self.email, self.phone, None)
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
            let mut optional_fields = BTreeMap::new();

            let mut name: Option<String> = None;
            let mut email: Option<String> = None;
            let mut phone: Option<String> = None;

            for (i, field) in record.iter().enumerate() {
                let value = field.trim();
                match headers.get(i) {
                    Some("name") => {
                        if !value.is_empty() {
                            name = Some(value.to_string());
                        }
                    }
                    Some("email") => {
                        if !value.is_empty() {
                            email = Some(value.to_string());
                        }
                    }
                    Some("phone") => {
                        if !value.is_empty() {
                            phone = Some(value.to_string());
                        }
                    }
                    Some(header) => {
                        if value.is_empty() {
                            continue;
                        }
                        optional_fields.insert(header.to_string(), value.to_string());
                    }
                    None => continue,
                }
            }

            let Some(name) = name else {
                // Skip records missing required fields.
                continue;
            };

            clients.push(NewClient::new(
                hub_id,
                name,
                email,
                phone,
                Some(optional_fields),
            ));
        }

        Ok(clients)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_client_form_to_new_client_normalizes_optional_fields() {
        let form = AddClientForm {
            name: "Alice".to_string(),
            email: Some("Alice@Example.COM".to_string()),
            phone: Some("+1 (415) 555-2671".to_string()),
        };

        let new_client = form.to_new_client(42);

        assert_eq!(new_client.hub_id, 42);
        assert_eq!(new_client.email.as_deref(), Some("alice@example.com"));
        assert_eq!(new_client.phone.as_deref(), Some("+14155552671"));
    }

    #[test]
    fn add_client_form_to_new_client_handles_missing_optionals() {
        let form = AddClientForm {
            name: "Bob".to_string(),
            email: None,
            phone: None,
        };

        let new_client = form.to_new_client(7);

        assert_eq!(new_client.hub_id, 7);
        assert!(new_client.email.is_none());
        assert!(new_client.phone.is_none());
    }
}
