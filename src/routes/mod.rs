//! Route modules wired into the Actix application.

use actix_web::{HttpResponse, http::StatusCode};
use pushkind_common::dto::mutation::ApiMutationErrorDto;

use crate::services::ServiceError;

pub mod api;
pub mod aux;
pub mod client;
pub mod main;
pub mod managers;
pub mod rate_limit;
pub mod settings;
pub mod store;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MutationResource {
    Client,
    ClientComment,
    ClientImport,
    Manager,
    Settings,
}

pub(crate) fn mutation_error_status(err: &ServiceError) -> StatusCode {
    match err {
        ServiceError::Form(_) | ServiceError::TypeConstraint(_) => StatusCode::BAD_REQUEST,
        ServiceError::Unauthorized => StatusCode::FORBIDDEN,
        ServiceError::NotFound => StatusCode::NOT_FOUND,
        ServiceError::Conflict => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn mutation_error_dto(resource: MutationResource, err: &ServiceError) -> ApiMutationErrorDto {
    match err {
        ServiceError::Form(message) => ApiMutationErrorDto {
            message: message.clone(),
            field_errors: Vec::new(),
        },
        ServiceError::TypeConstraint(message) => ApiMutationErrorDto {
            message: message.clone(),
            field_errors: Vec::new(),
        },
        ServiceError::Unauthorized => ApiMutationErrorDto {
            message: "Недостаточно прав.".to_string(),
            field_errors: Vec::new(),
        },
        ServiceError::NotFound => ApiMutationErrorDto {
            message: match resource {
                MutationResource::Client | MutationResource::ClientComment => "Клиент не найден.",
                MutationResource::Manager => "Менеджер не найден.",
                MutationResource::ClientImport | MutationResource::Settings => "Ресурс не найден.",
            }
            .to_string(),
            field_errors: Vec::new(),
        },
        ServiceError::Conflict => ApiMutationErrorDto {
            message: "Конфликт данных.".to_string(),
            field_errors: Vec::new(),
        },
        _ => ApiMutationErrorDto {
            message: "Внутренняя ошибка сервиса.".to_string(),
            field_errors: Vec::new(),
        },
    }
}

pub(crate) fn mutation_error_response(
    resource: MutationResource,
    err: &ServiceError,
) -> HttpResponse {
    HttpResponse::build(mutation_error_status(err)).json(mutation_error_dto(resource, err))
}
