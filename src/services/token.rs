use std::sync::Arc;

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::{AccessTokenClaims, RefreshTokenClaims};
use crate::services::SystemConfigService;

pub struct TokenService {
    pool: Arc<PgPool>,
    system_config: Arc<SystemConfigService>,
    access_token_ttl: i64,
    refresh_token_ttl: i64,
    auto_refresh_threshold: i64,
}

impl TokenService {
    pub fn new(
        pool: Arc<PgPool>,
        system_config: Arc<SystemConfigService>,
        access_token_ttl: i64,
        refresh_token_ttl: i64,
        auto_refresh_threshold: i64,
    ) -> Self {
        Self {
            pool,
            system_config,
            access_token_ttl,
            refresh_token_ttl,
            auto_refresh_threshold,
        }
    }

    pub async fn generate_access_token(&self, user_id: Uuid, email: &str) -> Result<String> {
        let secret = self.system_config.get_jwt_secret().await?;
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());

        let now = Utc::now();
        let exp = now + Duration::seconds(self.access_token_ttl);

        let claims = AccessTokenClaims {
            sub: user_id,
            email: email.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4(),
        };

        encode(&Header::default(), &claims, &encoding_key)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to generate token")))
    }

    pub async fn validate_access_token(&self, token: &str) -> Result<AccessTokenClaims> {
        let secret = self.system_config.get_jwt_secret().await?;
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let validation = Validation::default();
        let token_data = decode::<AccessTokenClaims>(token, &decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
                _ => AppError::InvalidToken,
            })?;

        Ok(token_data.claims)
    }

    pub fn should_refresh(&self, claims: &AccessTokenClaims) -> bool {
        let now = Utc::now().timestamp();
        claims.exp - now < self.auto_refresh_threshold
    }

    pub async fn generate_refresh_token(&self, user_id: Uuid) -> Result<String> {
        let secret = self.system_config.get_jwt_secret().await?;
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());

        let now = Utc::now();
        let exp = now + Duration::seconds(self.refresh_token_ttl);
        let jti = Uuid::new_v4();

        let claims = RefreshTokenClaims {
            sub: user_id,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti,
        };

        let token = encode(&Header::default(), &claims, &encoding_key)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to generate refresh token")))?;

        let token_hash = hash_token(&token);
        sqlx::query(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(&token_hash)
        .bind(exp)
        .execute(self.pool.as_ref())
        .await?;

        Ok(token)
    }

    pub async fn validate_refresh_token(&self, token: &str) -> Result<RefreshTokenClaims> {
        let secret = self.system_config.get_jwt_secret().await?;
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let validation = Validation::default();
        let token_data = decode::<RefreshTokenClaims>(token, &decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
                _ => AppError::InvalidToken,
            })?;

        let token_hash = hash_token(token);
        let exists: Option<(bool,)> = sqlx::query_as(
            "SELECT revoked FROM refresh_tokens WHERE token_hash = $1 AND expires_at > NOW()",
        )
        .bind(&token_hash)
        .fetch_optional(self.pool.as_ref())
        .await?;

        match exists {
            Some((true,)) => Err(AppError::TokenRevoked),
            Some((false,)) => Ok(token_data.claims),
            None => Err(AppError::InvalidToken),
        }
    }

    pub async fn revoke_refresh_token(&self, token: &str) -> Result<()> {
        let token_hash = hash_token(token);
        sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE token_hash = $1")
            .bind(&token_hash)
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
