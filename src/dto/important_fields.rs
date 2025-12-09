//! DTOs used by the important fields editor.

/// Data required to render the important fields management page.
#[derive(Debug)]
pub struct ImportantFieldsPageData {
    pub fields: Vec<String>,
}
