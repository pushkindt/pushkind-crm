use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::domain::types::{TypeConstraintError, normalize_phone_to_e164};

const PHONE_MAX_LEN: usize = 64;
const PHONE_MAX_LEN_VALIDATOR: u64 = PHONE_MAX_LEN as u64;

pub type StoreFormResult<T> = Result<T, StoreFormError>;

#[derive(Debug, Error)]
pub enum StoreFormError {
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
    #[error("phone number is required")]
    EmptyPhone,
    #[error("phone number is invalid")]
    InvalidPhone,
    #[error("otp must be a 6-digit code")]
    InvalidOtp,
}

#[derive(Debug, Deserialize, Validate)]
pub struct StoreOtpRequestPayload {
    #[validate(length(min = 1, max = PHONE_MAX_LEN_VALIDATOR))]
    pub phone: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoreOtpRequestInput {
    pub phone: String,
}

impl StoreOtpRequestPayload {
    pub fn into_request(self) -> StoreFormResult<StoreOtpRequestInput> {
        self.validate()?;

        let phone = normalize_phone_to_e164(&self.phone).map_err(|err| match err {
            TypeConstraintError::EmptyString => StoreFormError::EmptyPhone,
            TypeConstraintError::InvalidPhone => StoreFormError::InvalidPhone,
            _ => StoreFormError::InvalidPhone,
        })?;

        Ok(StoreOtpRequestInput { phone })
    }
}

#[derive(Debug, Deserialize)]
pub struct StoreOtpVerifyPayload {
    pub phone: String,
    pub otp: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreOtpVerifyInput {
    pub phone: String,
    pub otp: String,
}

impl StoreOtpVerifyPayload {
    pub fn into_request(self) -> StoreFormResult<StoreOtpVerifyInput> {
        let phone = normalize_phone_to_e164(&self.phone).map_err(|err| match err {
            TypeConstraintError::EmptyString => StoreFormError::EmptyPhone,
            TypeConstraintError::InvalidPhone => StoreFormError::InvalidPhone,
            _ => StoreFormError::InvalidPhone,
        })?;
        let otp = self.otp.trim();

        if otp.len() != 6 || !otp.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(StoreFormError::InvalidOtp);
        }

        Ok(StoreOtpVerifyInput {
            phone,
            otp: otp.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_otp_request_payload_trims_phone() {
        let payload = StoreOtpRequestPayload {
            phone: "  +1 (555) 123-4567  ".to_string(),
        };

        let normalized = payload.into_request().expect("valid payload");

        assert_eq!(normalized.phone, "+15551234567");
    }

    #[test]
    fn store_otp_verify_payload_rejects_invalid_otp() {
        let payload = StoreOtpVerifyPayload {
            phone: "+15551234567".to_string(),
            otp: "12ab".to_string(),
        };

        let result = payload.into_request();

        assert!(matches!(result, Err(StoreFormError::InvalidOtp)));
    }
}
