use std::future::{Ready, ready};

use actix_identity::Identity;
use actix_web::error::{ErrorInternalServerError, ErrorUnauthorized};
use actix_web::{Error, FromRequest, HttpRequest, dev::Payload, web::Data};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

use crate::models::config::ServerConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub sub: String, // subject (user ID or UUID)
    pub email: String,
    pub hub_id: i32,
    pub name: String,
    pub roles: Vec<String>,
    pub exp: usize, // expiration as timestamp
}

impl AuthenticatedUser {
    pub fn set_expiration(&mut self, days: i64) {
        let expiration = Utc::now()
            .checked_add_signed(Duration::days(days))
            .expect("valid timestamp")
            .timestamp() as usize;
        self.exp = expiration;
    }

    pub fn to_jwt(&mut self, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
        self.set_expiration(7);
        encode(
            &Header::default(),
            self,
            &EncodingKey::from_secret(secret.as_ref()),
        )
    }
    fn from_jwt(token: &str, secret: &str) -> Result<Self, jsonwebtoken::errors::Error> {
        let validation = jsonwebtoken::Validation::default();
        let token_data = jsonwebtoken::decode::<Self>(
            token,
            &DecodingKey::from_secret(secret.as_ref()),
            &validation,
        )?;
        Ok(token_data.claims)
    }
}

impl FromRequest for AuthenticatedUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let identity = Identity::from_request(req, &mut Payload::None)
            .into_inner()
            .map(|i| i.id().ok());

        let server_config = req.app_data::<Data<ServerConfig>>();

        let server_config = match server_config {
            Some(config) => config,
            None => return ready(Err(ErrorInternalServerError("Server config not found"))),
        };

        if let Ok(Some(uid)) = identity {
            let claims = AuthenticatedUser::from_jwt(&uid, &server_config.secret);

            match claims {
                Ok(claims) => return ready(Ok(claims)),
                Err(_) => return ready(Err(ErrorUnauthorized("Invalid user"))),
            };
        }
        ready(Err(ErrorUnauthorized("Unauthorized")))
    }
}
