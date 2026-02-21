use std::sync::Arc;

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use tokio::sync::RwLock;

use crate::models::AccessTokenClaims;
use crate::services::SystemConfigService;

pub struct JwtValidator {
    system_config: Arc<SystemConfigService>,
    validation: Validation,
    auto_refresh_threshold: i64,
    cached_secret: Arc<RwLock<String>>,
}

impl JwtValidator {
    pub fn new(system_config: Arc<SystemConfigService>, auto_refresh_threshold: i64) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_required_spec_claims(&["exp", "sub", "iat"]);

        Self {
            system_config,
            validation,
            auto_refresh_threshold,
            cached_secret: Arc::new(RwLock::new(String::new())),
        }
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        let secret = self.system_config.get_jwt_secret().await?;
        let mut cached = self.cached_secret.write().await;
        *cached = secret;
        Ok(())
    }

    pub async fn refresh_secret(&self) -> anyhow::Result<()> {
        self.system_config.invalidate_cache().await;
        self.init().await
    }

    pub async fn validate(&self, token: &str) -> Result<AccessTokenClaims, JwtError> {
        let secret = self.cached_secret.read().await;
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let token_data = decode::<AccessTokenClaims>(token, &decoding_key, &self.validation)
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
