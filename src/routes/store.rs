use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use log::{error, info};
use pushkind_common::zmq::ZmqSender;
use serde::Deserialize;
use serde_json::json;

use crate::forms::store::{StoreOtpRequestPayload, StoreOtpVerifyPayload};
use crate::models::config::AppConfig;
use crate::repository::DieselRepository;
use crate::routes::rate_limit::StoreOtpIpRateLimiter;
use crate::services::ServiceError;
use crate::services::store::{
    clear_store_session_cookie, decode_store_session_cookie, request_store_otp, verify_store_otp,
};

#[derive(Debug, Deserialize)]
struct HubPath {
    hub_id: String,
}

#[post("/{hub_id}/auth/otp")]
pub async fn request_store_auth_otp(
    req: HttpRequest,
    path: web::Path<HubPath>,
    payload: web::Json<StoreOtpRequestPayload>,
    repo: web::Data<DieselRepository>,
    sms_sender: web::Data<ZmqSender>,
    app_config: web::Data<AppConfig>,
    rate_limiter: web::Data<StoreOtpIpRateLimiter>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let request = match payload.into_inner().into_request() {
        Ok(request) => request,
        Err(error) => {
            return HttpResponse::UnprocessableEntity().json(json!({ "error": error.to_string() }));
        }
    };

    if let Some(response) = rate_limit_otp_response(&req, hub_id, rate_limiter.get_ref()) {
        return response;
    }

    match request_store_otp(
        hub_id,
        request,
        repo.get_ref(),
        sms_sender.get_ref(),
        &app_config.sms_sender,
    )
    .await
    {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Form(message)) => {
            HttpResponse::UnprocessableEntity().json(json!({ "error": message }))
        }
        Err(err) => {
            error!("Failed to process CRM OTP request for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn rate_limit_otp_response(
    req: &HttpRequest,
    hub_id: i32,
    rate_limiter: &StoreOtpIpRateLimiter,
) -> Option<HttpResponse> {
    let exceeded = match rate_limiter.check(req) {
        Ok(()) => return None,
        Err(err) => err,
    };

    let mut retry_after_seconds = exceeded.retry_after.as_secs();
    if exceeded.retry_after.subsec_nanos() > 0 {
        retry_after_seconds = retry_after_seconds.saturating_add(1);
    }
    retry_after_seconds = retry_after_seconds.max(1);

    info!(
        "CRM storefront OTP request rate limited for hub {hub_id} from ip {} (retry-after={retry_after_seconds}s)",
        exceeded.ip
    );

    Some(
        HttpResponse::TooManyRequests()
            .insert_header(("Retry-After", retry_after_seconds.to_string()))
            .json(json!({ "error": "rate limit exceeded" })),
    )
}

#[post("/{hub_id}/auth/otp/verify")]
pub async fn verify_store_auth_otp(
    path: web::Path<HubPath>,
    payload: web::Json<StoreOtpVerifyPayload>,
    repo: web::Data<DieselRepository>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let request = match payload.into_inner().into_request() {
        Ok(request) => request,
        Err(error) => {
            return HttpResponse::UnprocessableEntity().json(json!({ "error": error.to_string() }));
        }
    };

    match verify_store_otp(
        hub_id,
        request,
        repo.get_ref(),
        &app_config.secret,
        &app_config.domain,
    ) {
        Ok((response, cookie)) => HttpResponse::Ok().cookie(cookie).json(response),
        Err(ServiceError::Form(message)) => {
            HttpResponse::UnprocessableEntity().json(json!({ "error": message }))
        }
        Err(err) => {
            error!("Failed to verify CRM OTP for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/{hub_id}/auth/session")]
pub async fn get_store_session(
    path: web::Path<HubPath>,
    req: HttpRequest,
    repo: web::Data<DieselRepository>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    let hub_id = match path.into_inner().hub_id.parse::<i32>() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let Some(cookie) = req.cookie(crate::domain::store_session::STORE_SESSION_COOKIE_NAME) else {
        return HttpResponse::Unauthorized().finish();
    };

    match decode_store_session_cookie(cookie.value(), hub_id, repo.get_ref(), &app_config.secret) {
        Ok(customer) => HttpResponse::Ok().json(customer),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized()
            .cookie(clear_store_session_cookie(&app_config.domain))
            .finish(),
        Err(err) => {
            error!("Failed to validate CRM store session for hub {hub_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/{hub_id}/auth/logout")]
pub async fn logout_store_session(
    _path: web::Path<HubPath>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    HttpResponse::Ok()
        .cookie(clear_store_session_cookie(&app_config.domain))
        .json(json!({ "success": true }))
}
