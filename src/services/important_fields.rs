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

#[cfg(all(test, feature = "test-mocks"))]
mod tests {
    use super::*;
    use crate::domain::{important_field::ImportantField, types::HubId};
    use crate::repository::mock::MockRepository;
    use pushkind_common::services::errors::ServiceError;

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
        ImportantField::try_new(hub, name.to_string()).expect("valid important field")
    }

    /// Ensures loading fails for users lacking the admin role.
    #[test]
    fn load_requires_admin_role() {
        let mut repo = MockRepository::new();
        repo.expect_list_important_fields().times(0);
        let user = viewer_user();

        let result = load_important_fields(&repo, &user);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    /// Ensures saving fails for users lacking the admin role.
    #[test]
    fn save_requires_admin_role() {
        let mut repo = MockRepository::new();
        repo.expect_replace_important_fields().times(0);
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
        let mut repo = MockRepository::new();
        repo.expect_replace_important_fields()
            .withf(|hub_id, fields| {
                let expected_hub = HubId::new(42).expect("valid hub id");
                hub_id == &expected_hub
                    && fields.len() == 2
                    && fields[0].field.as_str() == "Name"
                    && fields[1].field.as_str() == "Phone"
            })
            .times(1)
            .returning(|_, _| Ok(()));
        let user = admin_user();
        let form = ImportantFieldsForm {
            fields: "Name\nPhone".to_string(),
        };

        save_important_fields(&repo, &user, form).expect("should save fields");
    }

    /// Checks that loading returns already saved field names.
    #[test]
    fn load_returns_existing_fields() {
        let mut repo = MockRepository::new();
        let expected_fields = vec![build_field(42, "Name"), build_field(42, "Email")];
        repo.expect_list_important_fields()
            .withf(|hub_id| hub_id == &HubId::new(42).expect("valid hub id"))
            .times(1)
            .returning(move |_| Ok(expected_fields.clone()));

        let user = admin_user();
        let data = load_important_fields(&repo, &user).expect("should load fields");

        assert_eq!(data.fields, vec!["Name", "Email"]);
    }
}
