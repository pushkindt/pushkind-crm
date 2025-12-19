//! Forms for managing important field definitions.

use serde::Deserialize;

use crate::{
    domain::{
        important_field::ImportantField,
        types::{HubId, ImportantFieldName},
    },
    forms::FormError,
};

/// Form capturing the textarea payload with important field names.
#[derive(Debug, Deserialize)]
pub struct ImportantFieldsForm {
    #[serde(default)]
    pub fields: String,
}

/// Payload representing the important fields as a vector of strings.
pub struct ImportantFieldsPayload {
    pub fields: Vec<ImportantFieldName>,
}

impl TryFrom<ImportantFieldsForm> for ImportantFieldsPayload {
    type Error = FormError;

    fn try_from(form: ImportantFieldsForm) -> Result<Self, Self::Error> {
        let mut fields = form
            .fields
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<&str>>();
        fields.sort_unstable();
        fields.dedup();
        let fields = fields
            .iter()
            .map(|&field| ImportantFieldName::new(field).map_err(|_| FormError::InvalidName))
            .collect::<Result<Vec<ImportantFieldName>, FormError>>()?;

        Ok(Self { fields })
    }
}

impl ImportantFieldsPayload {
    pub fn into_domain(self, hub_id: HubId) -> Vec<ImportantField> {
        self.fields
            .into_iter()
            .map(|field| ImportantField::new(hub_id, field))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_important_fields_form_to_payload() {
        let form = ImportantFieldsForm {
            fields: "  Field One  \n\nField Two\n  \nField Three  ".to_string(),
        };

        let payload = ImportantFieldsPayload::try_from(form).unwrap();

        let fields = payload
            .fields
            .iter()
            .map(|f| f.as_str().to_string())
            .collect::<Vec<String>>();

        assert_eq!(
            fields,
            vec![
                "Field One".to_string(),
                "Field Three".to_string(),
                "Field Two".to_string()
            ]
        );
    }
}
