use std::str::FromStr;

use actix_web::cookie::{Cookie, SameSite, time::Duration as CookieDuration};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use log::info;
use pushkind_common::zmq::{ZmqSenderExt, ZmqSenderTrait};
use pushkind_sms::models::zmq::ZMQSendSmsMessage;
use rand::RngExt;

use crate::domain::client::NewClient;
use crate::domain::store_otp::NewStoreOtp;
use crate::domain::store_session::{
    STORE_SESSION_COOKIE_NAME, STORE_SESSION_TTL_DAYS, StoreSessionClaims,
};
use crate::domain::types::{HubId, PhoneNumber, PublicId};
use crate::dto::store::{StoreOtpAcceptResponse, StoreOtpVerifyResponse, StoreSessionUser};
use crate::forms::store::{StoreOtpRequestPayload, StoreOtpVerifyPayload};
use crate::repository::{ClientReader, ClientWriter, StoreOtpRepository};
use crate::services::{ServiceError, ServiceResult};

const OTP_EXPIRY_MINUTES: i64 = 10;
const OTP_THROTTLE_MINUTES: i64 = 2;
const OTP_THROTTLE_MESSAGE: &str = "Подождите перед повторным запросом кода";
const OTP_INVALID_MESSAGE: &str = "Неверный или просроченный код";

pub async fn request_store_otp<R>(
    hub_id: i32,
    payload: StoreOtpRequestPayload,
    repo: &R,
    zmq_sender: &impl ZmqSenderTrait,
    sms_sender: &str,
) -> ServiceResult<StoreOtpAcceptResponse>
where
    R: StoreOtpRepository + ?Sized,
{
    let request = payload.into_request()?;
    let hub_id = HubId::new(hub_id)?;
    let phone = PhoneNumber::new(request.phone.clone())?;
    let now = Utc::now().naive_utc();

    if let Some(existing) = repo.get_store_otp(hub_id, &phone)?
        && existing.last_sent_at + Duration::minutes(OTP_THROTTLE_MINUTES) > now
    {
        return Err(ServiceError::Form(OTP_THROTTLE_MESSAGE.to_string()));
    }

    let code = format!("{:06}", rand::rng().random_range(0..1_000_000u32));
    let expires_at = now + Duration::minutes(OTP_EXPIRY_MINUTES);
    let otp_payload =
        NewStoreOtp::try_new(hub_id.get(), phone.as_str(), code.clone(), expires_at, now)?;

    repo.upsert_store_otp(&otp_payload)?;

    let zmq_message = ZMQSendSmsMessage {
        sender_id: sms_sender.to_string(),
        phone_number: phone.as_str().to_string(),
        message: format!("Your OTP is {code}"),
    };

    zmq_sender.send_json(&zmq_message).await?;

    info!(
        "CRM storefront OTP request accepted for hub {hub_id} and phone {}",
        phone.as_str()
    );

    Ok(StoreOtpAcceptResponse { success: true })
}

pub fn verify_store_otp<R>(
    hub_id: i32,
    payload: StoreOtpVerifyPayload,
    repo: &R,
    secret: &str,
    domain: &str,
) -> ServiceResult<(StoreOtpVerifyResponse, Cookie<'static>)>
where
    R: ClientReader + ClientWriter + StoreOtpRepository + ?Sized,
{
    let request = payload.into_request()?;
    let hub_id = HubId::new(hub_id)?;
    let phone = PhoneNumber::new(request.phone.clone())?;
    let now = Utc::now().naive_utc();

    let record = repo
        .get_store_otp(hub_id, &phone)?
        .ok_or_else(|| ServiceError::Form(OTP_INVALID_MESSAGE.to_string()))?;

    if record.code != request.otp || record.expires_at <= now {
        return Err(ServiceError::Form(OTP_INVALID_MESSAGE.to_string()));
    }

    let customer = match repo.get_client_by_phone(&phone, hub_id)? {
        Some(customer) => customer,
        None => {
            let new_client = NewClient::try_new(
                hub_id.get(),
                phone.as_str().to_string(),
                None,
                Some(phone.as_str().to_string()),
                None,
            )?;

            repo.create_or_replace_clients(&[new_client])?;
            repo.get_client_by_phone(&phone, hub_id)?
                .ok_or(ServiceError::Internal)?
        }
    };

    let claims = claims_from_client(&customer)?;
    let cookie = build_store_session_cookie(&claims, secret, domain)?;
    let dto = StoreSessionUser::try_from(customer).map_err(|_| ServiceError::Internal)?;
    repo.delete_store_otp(hub_id, &phone)?;

    Ok((
        StoreOtpVerifyResponse {
            success: true,
            customer: dto,
        },
        cookie,
    ))
}

fn claims_from_client(client: &crate::domain::client::Client) -> ServiceResult<StoreSessionClaims> {
    let public_id = client.public_id.ok_or(ServiceError::Internal)?;
    let phone = client.phone.clone().ok_or(ServiceError::Internal)?;

    let mut claims = StoreSessionClaims {
        sub: public_id.to_string(),
        hub_id: client.hub_id.get(),
        name: client.name.as_str().to_string(),
        phone: phone.as_str().to_string(),
        email: client
            .email
            .as_ref()
            .map(|email| email.as_str().to_string()),
        exp: 0,
    };
    claims.set_expiration(STORE_SESSION_TTL_DAYS);

    Ok(claims)
}

pub fn build_store_session_cookie(
    claims: &StoreSessionClaims,
    secret: &str,
    domain: &str,
) -> ServiceResult<Cookie<'static>> {
    let token = encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(|_| ServiceError::Internal)?;

    Ok(Cookie::build(STORE_SESSION_COOKIE_NAME, token)
        .domain(format!(".{domain}"))
        .path("/")
        .same_site(SameSite::Lax)
        .http_only(true)
        .max_age(CookieDuration::days(STORE_SESSION_TTL_DAYS))
        .finish())
}

pub fn clear_store_session_cookie(domain: &str) -> Cookie<'static> {
    Cookie::build(STORE_SESSION_COOKIE_NAME, "")
        .domain(format!(".{domain}"))
        .path("/")
        .same_site(SameSite::Lax)
        .http_only(true)
        .max_age(CookieDuration::seconds(0))
        .finish()
}

pub fn decode_store_session_cookie<R>(
    token: &str,
    hub_id: i32,
    repo: &R,
    secret: &str,
) -> ServiceResult<StoreSessionUser>
where
    R: ClientReader + ?Sized,
{
    let token_data = decode::<StoreSessionClaims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| ServiceError::Unauthorized)?;

    let claims = token_data.claims;
    if !claims.matches_hub(hub_id) {
        return Err(ServiceError::Unauthorized);
    }

    let public_id = PublicId::from_str(&claims.sub).map_err(|_| ServiceError::Unauthorized)?;
    let hub_id = HubId::new(claims.hub_id)?;
    let customer = repo
        .get_client_by_public_id(public_id, hub_id)?
        .ok_or(ServiceError::Unauthorized)?;

    StoreSessionUser::try_from(customer).map_err(|_| ServiceError::Internal)
}

#[cfg(all(test, feature = "test-mocks"))]
mod tests {
    use super::*;
    use crate::domain::client::Client;
    use crate::repository::mock::MockRepository;
    use mockall::Sequence;
    use pushkind_common::zmq::{SendFuture, ZmqSenderError};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct TestZmqSender {
        recorded: Arc<Mutex<Vec<ZMQSendSmsMessage>>>,
    }

    impl TestZmqSender {
        fn messages(&self) -> Arc<Mutex<Vec<ZMQSendSmsMessage>>> {
            self.recorded.clone()
        }
    }

    impl ZmqSenderTrait for TestZmqSender {
        fn send_bytes<'a>(&'a self, bytes: Vec<u8>) -> SendFuture<'a> {
            let recorded = self.recorded.clone();
            Box::pin(async move {
                let msg: ZMQSendSmsMessage =
                    serde_json::from_slice(&bytes).map_err(ZmqSenderError::from)?;
                recorded.lock().unwrap().push(msg);
                Ok(())
            })
        }

        fn try_send_bytes(&self, bytes: Vec<u8>) -> Result<(), ZmqSenderError> {
            let msg: ZMQSendSmsMessage =
                serde_json::from_slice(&bytes).map_err(ZmqSenderError::from)?;
            self.recorded.lock().unwrap().push(msg);
            Ok(())
        }

        fn send_multipart<'a>(&'a self, _frames: Vec<Vec<u8>>) -> SendFuture<'a> {
            Box::pin(async { Ok(()) })
        }
    }

    fn sample_client() -> Client {
        Client::try_new(
            1,
            Some(PublicId::new().as_bytes()),
            7,
            "Alice".to_string(),
            Some("alice@example.com".to_string()),
            Some("+15551234567".to_string()),
            Utc::now().naive_utc(),
            Utc::now().naive_utc(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn request_store_otp_accepts_first_request() {
        let mut repo = MockRepository::new();
        repo.expect_get_store_otp().returning(|_, _| Ok(None));
        repo.expect_upsert_store_otp().returning(|otp| {
            Ok(crate::domain::store_otp::StoreOtp {
                hub_id: otp.hub_id,
                phone: otp.phone.clone(),
                code: otp.code.clone(),
                expires_at: otp.expires_at,
                last_sent_at: otp.last_sent_at,
            })
        });
        let sender = TestZmqSender::default();

        let response = actix_web::rt::System::new()
            .block_on(async {
                request_store_otp(
                    7,
                    StoreOtpRequestPayload {
                        phone: "+15551234567".to_string(),
                    },
                    &repo,
                    &sender,
                    "sender-id",
                )
                .await
            })
            .expect("otp accepted");

        assert!(response.success);
        let recorded = sender.messages();
        let messages = recorded.lock().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].phone_number, "+15551234567");
    }

    #[test]
    fn verify_store_otp_creates_cookie_and_customer_payload() {
        let mut repo = MockRepository::new();
        let client = sample_client();
        let mut sequence = Sequence::new();
        repo.expect_get_store_otp().returning(|_, _| {
            Ok(Some(crate::domain::store_otp::StoreOtp {
                hub_id: HubId::new(7).unwrap(),
                phone: PhoneNumber::new("+15551234567").unwrap(),
                code: "123456".to_string(),
                expires_at: Utc::now().naive_utc() + Duration::minutes(10),
                last_sent_at: Utc::now().naive_utc(),
            }))
        });
        repo.expect_get_client_by_phone()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(move |_, _| Ok(Some(client.clone())));
        repo.expect_delete_store_otp()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_, _| Ok(()));

        let (response, cookie) = verify_store_otp(
            7,
            StoreOtpVerifyPayload {
                phone: "+15551234567".to_string(),
                otp: "123456".to_string(),
            },
            &repo,
            "secret",
            "example.com",
        )
        .expect("verified");

        assert!(response.success);
        assert_eq!(response.customer.phone, "+15551234567");
        assert_eq!(cookie.name(), STORE_SESSION_COOKIE_NAME);
    }

    #[test]
    fn verify_store_otp_keeps_otp_when_customer_creation_fails() {
        let mut repo = MockRepository::new();
        let mut sequence = Sequence::new();
        repo.expect_get_store_otp().returning(|_, _| {
            Ok(Some(crate::domain::store_otp::StoreOtp {
                hub_id: HubId::new(7).unwrap(),
                phone: PhoneNumber::new("+15551234567").unwrap(),
                code: "123456".to_string(),
                expires_at: Utc::now().naive_utc() + Duration::minutes(10),
                last_sent_at: Utc::now().naive_utc(),
            }))
        });
        repo.expect_get_client_by_phone()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_, _| Ok(None));
        repo.expect_create_or_replace_clients()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_| Ok(0));
        repo.expect_get_client_by_phone()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_, _| Ok(None));
        repo.expect_delete_store_otp().never();

        let result = verify_store_otp(
            7,
            StoreOtpVerifyPayload {
                phone: "+15551234567".to_string(),
                otp: "123456".to_string(),
            },
            &repo,
            "secret",
            "example.com",
        );

        assert!(matches!(result, Err(ServiceError::Internal)));
    }

    #[test]
    fn decode_store_session_cookie_rejects_hub_mismatch() {
        let claims = StoreSessionClaims {
            sub: PublicId::new().to_string(),
            hub_id: 7,
            name: "Alice".to_string(),
            phone: "+15551234567".to_string(),
            email: None,
            exp: (Utc::now() + Duration::days(1)).timestamp() as usize,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("secret".as_bytes()),
        )
        .unwrap();
        let repo = MockRepository::new();

        let result = decode_store_session_cookie(&token, 8, &repo, "secret");

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }
}
