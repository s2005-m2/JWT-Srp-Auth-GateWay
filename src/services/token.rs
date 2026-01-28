use std::sync::Arc;

use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::{AccessTokenClaims, RefreshTokenClaims};
use crate::services::SystemConfigService;

type HmacSha256 = Hmac<Sha256>;

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

        let token_hash = Self::hmac_hash_token(&token, &secret);
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

        let token_hash = Self::hmac_hash_token(token, &secret);
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
        let secret = self.system_config.get_jwt_secret().await?;
        let token_hash = Self::hmac_hash_token(token, &secret);
        sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE token_hash = $1")
            .bind(&token_hash)
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }

    fn hmac_hash_token(token: &str, secret: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .unwrap_or_else(|_| {
                tracing::error!("HMAC initialization failed - this should never happen");
                panic!("HMAC initialization failed with valid key")
            });
        mac.update(token.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_hash_consistency() {
        let token = "test_token_12345";
        let secret = "my_secret_key";
        
        let hash1 = TokenService::hmac_hash_token(token, secret);
        let hash2 = TokenService::hmac_hash_token(token, secret);
        
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hmac_hash_different_tokens() {
        let secret = "my_secret_key";
        
        let hash1 = TokenService::hmac_hash_token("token_a", secret);
        let hash2 = TokenService::hmac_hash_token("token_b", secret);
        
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hmac_hash_different_secrets() {
        let token = "same_token";
        
        let hash1 = TokenService::hmac_hash_token(token, "secret_1");
        let hash2 = TokenService::hmac_hash_token(token, "secret_2");
        
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hmac_hash_output_format() {
        let hash = TokenService::hmac_hash_token("token", "secret");
        
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
