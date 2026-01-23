use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::{Admin, AdminRegistrationToken, AdminTokenClaims};

pub struct AdminService {
    pool: Arc<PgPool>,
    jwt_secret: String,
}

impl AdminService {
    pub fn new(pool: Arc<PgPool>, jwt_secret: String) -> Self {
        Self { pool, jwt_secret }
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<Admin>> {
        let admin = sqlx::query_as::<_, Admin>("SELECT * FROM admins WHERE username = $1")
            .bind(username)
            .fetch_optional(self.pool.as_ref())
            .await?;
        Ok(admin)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Admin>> {
        let admin = sqlx::query_as::<_, Admin>("SELECT * FROM admins WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool.as_ref())
            .await?;
        Ok(admin)
    }

    pub async fn count(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM admins")
            .fetch_one(self.pool.as_ref())
            .await?;
        Ok(count.0)
    }

    pub async fn create_with_token(
        &self,
        username: &str,
        password: &str,
        registration_token: &str,
    ) -> Result<Admin> {
        let token_hash = hash_token(registration_token);
        
        let token_record = sqlx::query_as::<_, AdminRegistrationToken>(
            "SELECT * FROM admin_registration_tokens 
             WHERE token_hash = $1 AND used = FALSE AND expires_at > NOW()"
        )
        .bind(&token_hash)
        .fetch_optional(self.pool.as_ref())
        .await?;

        let token_record = token_record.ok_or(AppError::InvalidToken)?;
        let password_hash = hash_password(password)?;

        let admin = sqlx::query_as::<_, Admin>(
            "INSERT INTO admins (username, password_hash) VALUES ($1, $2) RETURNING *"
        )
        .bind(username)
        .bind(&password_hash)
        .fetch_one(self.pool.as_ref())
        .await?;

        sqlx::query(
            "UPDATE admin_registration_tokens SET used = TRUE, used_by = $1 WHERE id = $2"
        )
        .bind(admin.id)
        .bind(token_record.id)
        .execute(self.pool.as_ref())
        .await?;

        Ok(admin)
    }

    pub fn verify_password(&self, admin: &Admin, password: &str) -> Result<bool> {
        verify_password(password, &admin.password_hash)
    }

    pub async fn generate_registration_token(&self) -> Result<String> {
        let token = Uuid::new_v4().to_string();
        let token_hash = hash_token(&token);
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

        sqlx::query(
            "INSERT INTO admin_registration_tokens (token_hash, expires_at) VALUES ($1, $2)"
        )
        .bind(&token_hash)
        .bind(expires_at)
        .execute(self.pool.as_ref())
        .await?;

        Ok(token)
    }

    pub async fn has_valid_registration_token(&self) -> Result<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM admin_registration_tokens WHERE used = FALSE AND expires_at > NOW()"
        )
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(count.0 > 0)
    }

    pub fn generate_admin_jwt(&self, admin: &Admin) -> Result<String> {
        use jsonwebtoken::{encode, EncodingKey, Header};
        
        const ADMIN_TOKEN_TTL_SECS: i64 = 86400;
        let now = chrono::Utc::now().timestamp();
        let exp = now + ADMIN_TOKEN_TTL_SECS;
        
        let claims = AdminTokenClaims {
            sub: admin.id,
            username: admin.username.clone(),
            role: "admin".to_string(),
            exp,
            iat: now,
            jti: Uuid::new_v4(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|_| AppError::Internal(anyhow::anyhow!("JWT encoding failed")))?;

        Ok(token)
    }

    pub fn validate_admin_jwt(&self, token: &str) -> Result<AdminTokenClaims> {
        use jsonwebtoken::{decode, DecodingKey, Validation};

        let token_data = decode::<AdminTokenClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::InvalidToken)?;

        Ok(token_data.claims)
    }
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Password hashing failed")))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid password hash")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
