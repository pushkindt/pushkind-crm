//! Form definitions backing the CRM routes.

use std::borrow::Cow;

use thiserror::Error;
use validator::{ValidationError, ValidationErrors};

pub mod client;
pub mod important_fields;
pub mod main;
pub mod managers;
pub mod store;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormFieldError {
    pub field: Cow<'static, str>,
    pub message: Cow<'static, str>,
}

#[derive(Debug, Error)]
/// Errors that can occur when processing form data.
pub enum FormError {
    #[error("{}", validation_errors_display(.0))]
    Validation(#[from] ValidationErrors),

    #[error("Укажите корректный электронный адрес.")]
    InvalidEmail,

    #[error("Выберите хаб.")]
    InvalidHubId,

    #[error("Выберите менеджера.")]
    InvalidManagerId,

    #[error("Выберите корректных клиентов.")]
    InvalidClientId,

    #[error("Укажите имя.")]
    InvalidName,

    #[error("Укажите корректный номер телефона.")]
    InvalidPhoneNumber,

    #[error("Укажите корректный URL.")]
    InvalidUrl,

    #[error("Укажите электронный адрес или телефон.")]
    MissingClientContact,

    #[error("Введите сообщение.")]
    InvalidCommentMessage,

    #[error("Тема заполнена некорректно.")]
    InvalidCommentSubject,

    #[error("Укажите название вложения.")]
    InvalidAttachmentName,

    #[error("Название поля заполнено некорректно.")]
    InvalidImportantFieldName,
}

impl FormError {
    pub(crate) fn field_errors(&self) -> Vec<FormFieldError> {
        match self {
            Self::Validation(errors) => collect_validation_errors(errors),
            _ => self
                .field()
                .map(|field| vec![field_error(field, self.to_string())])
                .unwrap_or_default(),
        }
    }

    fn field(&self) -> Option<&'static str> {
        match self {
            Self::Validation(_) => None,
            Self::InvalidEmail => Some("email"),
            Self::InvalidHubId => Some("hub_id"),
            Self::InvalidManagerId => Some("manager_id"),
            Self::InvalidClientId => Some("client_ids"),
            Self::InvalidName => Some("name"),
            Self::InvalidPhoneNumber => Some("phone"),
            Self::InvalidUrl => Some("url"),
            Self::MissingClientContact => Some("email"),
            Self::InvalidCommentMessage => Some("message"),
            Self::InvalidCommentSubject => Some("subject"),
            Self::InvalidAttachmentName => Some("text"),
            Self::InvalidImportantFieldName => Some("fields"),
        }
    }
}

fn collect_validation_errors(errors: &ValidationErrors) -> Vec<FormFieldError> {
    errors
        .field_errors()
        .iter()
        .flat_map(|(field, field_errors)| {
            field_errors.iter().map(|error| FormFieldError {
                field: field.clone(),
                message: validation_error_message(error),
            })
        })
        .collect()
}

fn validation_error_message(error: &ValidationError) -> Cow<'static, str> {
    error
        .message
        .clone()
        .unwrap_or(Cow::Borrowed("Поле заполнено некорректно."))
}

fn validation_errors_display(errors: &ValidationErrors) -> String {
    let messages = collect_validation_errors(errors)
        .into_iter()
        .map(|error| error.message.into_owned())
        .collect::<Vec<_>>();

    if messages.is_empty() {
        "Ошибка валидации формы.".to_string()
    } else {
        format!("Ошибка валидации формы: {}", messages.join("; "))
    }
}

fn field_error(field: &'static str, message: impl Into<Cow<'static, str>>) -> FormFieldError {
    FormFieldError {
        field: Cow::Borrowed(field),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::FormError;
    use crate::forms::client::{AddAttachmentForm, AddCommentForm, SaveClientForm};
    use crate::forms::main::AddClientForm;
    use crate::forms::managers::AddManagerForm;
    use validator::Validate;

    fn field_errors(error: &FormError) -> Vec<(String, String)> {
        let mut field_errors = error
            .field_errors()
            .into_iter()
            .map(|error| (error.field.to_string(), error.message.into_owned()))
            .collect::<Vec<_>>();
        field_errors.sort();
        field_errors
    }

    #[test]
    fn validation_errors_use_messages_declared_by_crm_forms() {
        let form = AddClientForm {
            name: String::new(),
            email: Some("invalid".to_string()),
            phone: None,
        };

        let error = FormError::from(form.validate().expect_err("form should be invalid"));

        assert_eq!(
            field_errors(&error),
            vec![
                (
                    "email".to_string(),
                    "Укажите корректный электронный адрес.".to_string(),
                ),
                ("name".to_string(), "Укажите имя.".to_string()),
            ]
        );
    }

    #[test]
    fn manager_and_client_form_messages_stay_localized() {
        let manager_error = FormError::from(
            AddManagerForm {
                name: String::new(),
                email: "invalid".to_string(),
            }
            .validate()
            .expect_err("form should be invalid"),
        );

        assert_eq!(
            field_errors(&manager_error),
            vec![
                (
                    "email".to_string(),
                    "Укажите корректный электронный адрес.".to_string(),
                ),
                ("name".to_string(), "Укажите имя.".to_string()),
            ]
        );

        let client_error = FormError::from(
            SaveClientForm {
                name: String::new(),
                email: Some("invalid".to_string()),
                phone: None,
                field: Vec::new(),
                value: Vec::new(),
            }
            .validate()
            .expect_err("form should be invalid"),
        );

        assert_eq!(
            field_errors(&client_error),
            vec![
                (
                    "email".to_string(),
                    "Укажите корректный электронный адрес.".to_string(),
                ),
                ("name".to_string(), "Укажите имя.".to_string()),
            ]
        );
    }

    #[test]
    fn attachment_and_comment_validation_messages_come_from_forms() {
        let attachment_error = FormError::from(
            AddAttachmentForm {
                text: String::new(),
                url: "invalid-url".to_string(),
            }
            .validate()
            .expect_err("form should be invalid"),
        );

        assert_eq!(
            field_errors(&attachment_error),
            vec![
                ("text".to_string(), "Укажите название вложения.".to_string()),
                ("url".to_string(), "Укажите корректный URL.".to_string()),
            ]
        );

        let comment_error = FormError::from(
            AddCommentForm {
                subject: None,
                message: String::new(),
                event_type: String::new(),
            }
            .validate()
            .expect_err("form should be invalid"),
        );

        assert_eq!(
            field_errors(&comment_error),
            vec![
                (
                    "event_type".to_string(),
                    "Выберите тип события.".to_string()
                ),
                ("message".to_string(), "Введите сообщение.".to_string()),
            ]
        );
    }

    #[test]
    fn conversion_error_messages_stay_in_forms_layer() {
        assert_eq!(
            field_errors(&FormError::MissingClientContact),
            vec![(
                "email".to_string(),
                "Укажите электронный адрес или телефон.".to_string(),
            )]
        );
        assert_eq!(
            field_errors(&FormError::InvalidAttachmentName),
            vec![("text".to_string(), "Укажите название вложения.".to_string(),)]
        );
        assert_eq!(
            FormError::InvalidCommentMessage.to_string(),
            "Введите сообщение."
        );
    }
}
