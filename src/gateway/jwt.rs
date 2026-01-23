use jsonwebtoken::{decode, DecodingKey, Validation};

use crate::config::JwtConfig;
use crate::models::AccessTokenClaims;

pub struct JwtValidator {
    decoding_key: DecodingKey,
    validation: Validation,
    auto_refresh_threshold: i64,
}

impl JwtValidator {
    pub fn new(config: &JwtConfig) -> Self {
        Self {
            decoding_key: DecodingKey::from_secret(config.secret.as_bytes()),
            validation: Validation::default(),
            auto_refresh_threshold: config.auto_refresh_threshold,
        }
    }

    pub fn validate(&self, token: &str) -> Result<AccessTokenClaims, JwtError> {
        let token_data = decode::<AccessTokenClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
            _ => JwtError::Invalid,
        })?;
        Ok(token_data.claims)
    }

    pub fn should_refresh(&self, claims: &AccessTokenClaims) -> bool {
        let now = chrono::Utc::now().timestamp();
        claims.exp - now < self.auto_refresh_threshold
    }
}

#[derive(Debug)]
pub enum JwtError {
    Invalid,
    Expired,
}
