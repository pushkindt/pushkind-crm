use actix_multipart::form::{MultipartForm, tempfile::TempFile};
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

#[derive(MultipartForm)]
pub struct UploadClientsForm {
    #[multipart(limit = "10MB")]
    pub csv: TempFile,
}

// impl UploadClientsForm {
//     pub fn parse_clients_csv(&self) -> Result<Vec<NewClient>, Box<dyn std::error::Error>> {
//         let mut rdr = csv::Reader::from_reader(csv.as_bytes());

//         let headers = rdr.headers()?.clone();
//         let mut recipients = Vec::new();

//         for result in rdr.records() {
//             let record = result?;
//             let mut optional_fields = HashMap::new();

//             let mut name = String::new();
//             let mut email = String::new();
//             let mut groups = Vec::new();

//             for (i, field) in record.iter().enumerate() {
//                 match headers.get(i) {
//                     Some("name") => name = field.to_string(),
//                     Some("email") => email = field.to_string(),
//                     Some("groups") => {
//                         groups = field
//                             .split(',')
//                             .map(|s| s.trim().to_string())
//                             .filter(|s| !s.is_empty())
//                             .collect();
//                     }
//                     Some(header) => {
//                         if field.len() == 0 {
//                             continue;
//                         }
//                         optional_fields.insert(header.to_string(), field.to_string());
//                     }
//                     None => continue,
//                 }
//             }

//             recipients.push(RecipientCSV {
//                 name,
//                 email,
//                 groups,
//                 optional_fields,
//             });
//         }

//         Ok(recipients)
//     }
// }
