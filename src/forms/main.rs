//! Forms tied to the main CRM dashboard.

use std::{collections::BTreeMap, io::Read};

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use crate::domain::client::NewClient;
use crate::domain::types::{ClientEmail, ClientName, HubId, PhoneNumber, TypeConstraintError};
use crate::forms::FormError;

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

pub struct AddClientPayload {
    pub name: ClientName,
    pub email: Option<ClientEmail>,
    pub phone: Option<PhoneNumber>,
}

impl TryFrom<AddClientForm> for AddClientPayload {
    type Error = FormError;

    fn try_from(form: AddClientForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        let name = ClientName::new(form.name).map_err(|_| FormError::InvalidName)?;
        let email = form
            .email
            .map(ClientEmail::try_from)
            .transpose()
            .map_err(|_| FormError::InvalidEmail)?;
        let phone = match form.phone {
            Some(value) => {
                Some(PhoneNumber::try_from(value).map_err(|_| FormError::InvalidPhoneNumber)?)
            }
            None => None,
        };

        if email.is_none() && phone.is_none() {
            Err(FormError::InvalidEmail)
        } else {
            Ok(Self { name, email, phone })
        }
    }
}

impl AddClientPayload {
    pub fn into_domain(self, hub_id: HubId) -> NewClient {
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
    #[error("Invalid client data: {0}")]
    ValidationError(#[from] TypeConstraintError),
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
    pub fn parse(&mut self, hub_id: HubId) -> Result<Vec<NewClient>, UploadClientsFormError> {
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

            let name = match ClientName::new(name) {
                Ok(name) => name,
                Err(_) => continue,
            };

            let email = email
                .map(ClientEmail::try_from)
                .and_then(|result| result.ok());

            let phone = match phone {
                Some(value) => Some(PhoneNumber::try_from(value)?),
                None => None,
            };

            if email.is_none() && phone.is_none() {
                continue;
            }

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

        let payload = AddClientPayload::try_from(form).expect("expected valid payload");

        let hub_id = HubId::new(42).expect("valid hub id");

        let new_client = payload.into_domain(hub_id);

        assert_eq!(new_client.hub_id.get(), 42);
        assert_eq!(
            new_client.email.as_ref().map(|email| email.as_str()),
            Some("alice@example.com")
        );
        assert_eq!(
            new_client.phone.as_ref().map(|phone| phone.as_str()),
            Some("+14155552671")
        );
    }

    #[test]
    fn add_client_form_to_new_client_handles_missing_optionals() {
        let form = AddClientForm {
            name: "Bob".to_string(),
            email: None,
            phone: None,
        };

        let payload = AddClientPayload::try_from(form);

        assert!(payload.is_err())
    }
}
