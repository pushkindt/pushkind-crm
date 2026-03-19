//! Storefront OTP domain models for CRM-backed customer authentication.

use chrono::NaiveDateTime;

use crate::domain::types::{HubId, PhoneNumber, TypeConstraintError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreOtp {
    pub hub_id: HubId,
    pub phone: PhoneNumber,
    pub code: String,
    pub expires_at: NaiveDateTime,
    pub last_sent_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewStoreOtp {
    pub hub_id: HubId,
    pub phone: PhoneNumber,
    pub code: String,
    pub expires_at: NaiveDateTime,
    pub last_sent_at: NaiveDateTime,
}

impl NewStoreOtp {
    #[must_use]
    pub fn new(
        hub_id: HubId,
        phone: PhoneNumber,
        code: String,
        expires_at: NaiveDateTime,
        last_sent_at: NaiveDateTime,
    ) -> Self {
        Self {
            hub_id,
            phone,
            code,
            expires_at,
            last_sent_at,
        }
    }

    pub fn try_new(
        hub_id: i32,
        phone: impl Into<String>,
        code: impl Into<String>,
        expires_at: NaiveDateTime,
        last_sent_at: NaiveDateTime,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::new(hub_id)?,
            PhoneNumber::new(phone)?,
            code.into(),
            expires_at,
            last_sent_at,
        ))
    }
}
