//! Services coordinating important field workflows.

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::ensure_role;

use crate::SERVICE_ADMIN_ROLE;
use crate::domain::types::HubId;
use crate::dto::important_fields::ImportantFieldsPageData;
use crate::forms::important_fields::{ImportantFieldsForm, ImportantFieldsPayload};
use crate::repository::{ImportantFieldReader, ImportantFieldWriter};
use crate::services::ServiceResult;

/// Loads the existing important field names for the admin interface.
pub fn load_important_fields<R>(
    repo: &R,
    user: &AuthenticatedUser,
) -> ServiceResult<ImportantFieldsPageData>
where
    R: ImportantFieldReader + ?Sized,
{
    ensure_role(user, SERVICE_ADMIN_ROLE)?;

    let hub_id = HubId::new(user.hub_id)?;

    let fields = repo
        .list_important_fields(hub_id)
        .map_err(|err| {
            log::error!("Failed to load important fields: {err}");
            err
        })?
        .into_iter()
        .map(|field| field.field.as_str().to_string())
        .collect();

    Ok(ImportantFieldsPageData { fields })
}

/// Persists the sanitized list of important field names for the hub.
pub fn save_important_fields<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: ImportantFieldsForm,
) -> ServiceResult<()>
where
    R: ImportantFieldWriter + ?Sized,
{
    ensure_role(user, SERVICE_ADMIN_ROLE)?;

    let payload = ImportantFieldsPayload::try_from(form)?;

    let hub_id = HubId::new(user.hub_id)?;
    let fields = payload.into_domain(hub_id);

    repo.replace_important_fields(hub_id, &fields)
        .map_err(|err| {
            log::error!("Failed to save important fields: {err}");
            err
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::domain::{
        important_field::ImportantField,
        types::{HubId, ImportantFieldName},
    };
    use pushkind_common::{repository::errors::RepositoryResult, services::errors::ServiceError};

    #[derive(Default)]
    struct MockRepo {
        stored: RefCell<Vec<ImportantField>>,
    }

    impl ImportantFieldReader for MockRepo {
        /// Returns the fields currently stored in the mock repository.
        fn list_important_fields(&self, _hub_id: HubId) -> RepositoryResult<Vec<ImportantField>> {
            Ok(self.stored.borrow().clone())
        }
    }

    impl ImportantFieldWriter for MockRepo {
        /// Replaces the stored fields in the mock repository.
        fn replace_important_fields(
            &self,
            _hub_id: HubId,
            fields: &[ImportantField],
        ) -> RepositoryResult<()> {
            self.stored.replace(fields.to_vec());
            Ok(())
        }
    }

    /// Builds an admin user for test scenarios.
    fn admin_user() -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "1".to_string(),
            email: "admin@example.com".to_string(),
            hub_id: 42,
            name: "Admin".to_string(),
            roles: vec![SERVICE_ADMIN_ROLE.to_string()],
            exp: 0,
        }
    }

    /// Builds a viewer user without admin rights.
    fn viewer_user() -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "2".to_string(),
            email: "viewer@example.com".to_string(),
            hub_id: 42,
            name: "Viewer".to_string(),
            roles: vec!["crm".to_string()],
            exp: 0,
        }
    }

    fn build_field(hub: i32, name: &str) -> ImportantField {
        ImportantField::new(
            HubId::new(hub).expect("valid hub id"),
            ImportantFieldName::new(name).expect("valid field name"),
        )
    }

    /// Ensures loading fails for users lacking the admin role.
    #[test]
    fn load_requires_admin_role() {
        let repo = MockRepo::default();
        let user = viewer_user();

        let result = load_important_fields(&repo, &user);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    /// Ensures saving fails for users lacking the admin role.
    #[test]
    fn save_requires_admin_role() {
        let repo = MockRepo::default();
        let user = viewer_user();
        let form = ImportantFieldsForm {
            fields: "Field".to_string(),
        };

        let result = save_important_fields(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    /// Verifies that normalization trims, sanitizes, and deduplicates input.
    #[test]
    fn normalize_fields_trims_sanitizes_and_deduplicates() {
        let form = ImportantFieldsForm {
            fields: "  Name  \n\nName\n\n Company ".to_string(),
        };

        let payload = ImportantFieldsPayload::try_from(form).expect("should normalize fields");

        let hub_id = HubId::new(7).expect("valid hub id");
        let fields = payload.into_domain(hub_id);

        let names: Vec<_> = fields
            .into_iter()
            .map(|f| f.field.as_str().to_string())
            .collect();

        assert_eq!(names, vec!["Company", "Name"]);
    }

    /// Confirms saving replaces the stored important fields.
    #[test]
    fn save_replaces_existing_fields() {
        let repo = MockRepo::default();
        let user = admin_user();
        let form = ImportantFieldsForm {
            fields: "Name\nPhone".to_string(),
        };

        save_important_fields(&repo, &user, form).expect("should save fields");

        let stored = repo.stored.borrow().clone();
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].field.as_str(), "Name");
        assert_eq!(stored[1].field.as_str(), "Phone");
    }

    /// Checks that loading returns already saved field names.
    #[test]
    fn load_returns_existing_fields() {
        let repo = MockRepo::default();
        repo.stored
            .replace(vec![build_field(42, "Name"), build_field(42, "Email")]);

        let user = admin_user();
        let data = load_important_fields(&repo, &user).expect("should load fields");

        assert_eq!(data.fields, vec!["Name", "Email"]);
    }
}
