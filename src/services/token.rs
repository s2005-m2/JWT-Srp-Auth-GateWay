use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::JwtConfig;
use crate::error::{AppError, Result};
use crate::models::AccessTokenClaims;

pub struct TokenService {
    pool: Arc<PgPool>,
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl TokenService {
    pub fn new(pool: Arc<PgPool>, config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());
        Self {
            pool,
            config,
            encoding_key,
            decoding_key,
        }
    }

    pub fn generate_access_token(&self, user_id: Uuid, email: &str) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.config.access_token_ttl);

        let claims = AccessTokenClaims {
            sub: user_id,
            email: email.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to generate token")))
    }

    pub fn validate_access_token(&self, token: &str) -> Result<AccessTokenClaims> {
        let validation = Validation::default();
        let token_data = decode::<AccessTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
                _ => AppError::InvalidToken,
            })?;

        Ok(token_data.claims)
    }

    pub fn should_refresh(&self, claims: &AccessTokenClaims) -> bool {
        let now = Utc::now().timestamp();
        claims.exp - now < self.config.auto_refresh_threshold
    }
}
