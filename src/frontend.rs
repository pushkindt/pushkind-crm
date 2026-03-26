//! Helpers for serving compiled frontend HTML documents.

use std::path::Path;

use actix_files::NamedFile;
use thiserror::Error;

/// Errors raised while opening built frontend documents.
#[derive(Debug, Error)]
pub enum FrontendAssetError {
    #[error("failed to open frontend document: {0}")]
    Read(#[from] std::io::Error),
}

/// Open a Vite-built HTML document for a React-owned route.
pub async fn open_frontend_html(path: impl AsRef<Path>) -> Result<NamedFile, FrontendAssetError> {
    let file = NamedFile::open_async(path).await?;
    Ok(file.use_last_modified(true).prefer_utf8(true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_open_existing_file() {
        let current_file_path: &'static str = file!();

        let result = actix_web::rt::System::new().block_on(open_frontend_html(current_file_path));
        assert!(result.is_ok());
    }

    #[test]
    fn missing_document_returns_read_error() {
        let error = actix_web::rt::System::new()
            .block_on(open_frontend_html("assets/dist/does-not-exist.html"))
            .unwrap_err();

        assert!(matches!(error, FrontendAssetError::Read(_)));
    }
}
