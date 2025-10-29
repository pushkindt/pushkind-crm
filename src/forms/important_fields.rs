use serde::Deserialize;

/// Form capturing the textarea payload with important field names.
#[derive(Debug, Deserialize)]
pub struct ImportantFieldsForm {
    #[serde(default)]
    pub fields: String,
}
