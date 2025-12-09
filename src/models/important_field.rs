//! Diesel models for important field records.

use diesel::prelude::*;
use serde::Serialize;

use crate::domain::important_field::ImportantField as DomainImportantField;

#[derive(Debug, Clone, Identifiable, Queryable, Selectable, Insertable, Serialize)]
#[diesel(table_name = crate::schema::important_fields)]
#[diesel(primary_key(hub_id, field))]
pub struct ImportantField {
    pub hub_id: i32,
    pub field: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::important_fields)]
pub struct NewImportantField<'a> {
    pub hub_id: i32,
    pub field: &'a str,
}

impl From<ImportantField> for DomainImportantField {
    fn from(value: ImportantField) -> Self {
        Self {
            hub_id: value.hub_id,
            field: value.field,
        }
    }
}

impl<'a> From<&'a DomainImportantField> for NewImportantField<'a> {
    fn from(value: &'a DomainImportantField) -> Self {
        Self {
            hub_id: value.hub_id,
            field: value.field.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::important_field::ImportantField as DomainImportantField;

    #[test]
    fn converts_from_db_to_domain() {
        let db = ImportantField {
            hub_id: 7,
            field: "Priority".to_string(),
        };

        let domain: DomainImportantField = db.into();

        assert_eq!(domain.hub_id, 7);
        assert_eq!(domain.field, "Priority");
    }

    #[test]
    fn converts_from_domain_to_insertable() {
        let domain = DomainImportantField::new(9, "Stage".to_string());

        let insertable: NewImportantField = (&domain).into();

        assert_eq!(insertable.hub_id, 9);
        assert_eq!(insertable.field, "Stage");
    }
}
