use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::User;

pub struct UserService {
    pool: Arc<PgPool>,
}

impl UserService {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(self.pool.as_ref())
            .await?;
        Ok(user)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool.as_ref())
            .await?;
        Ok(user)
    }

    pub async fn create(&self, email: &str, password: &str) -> Result<User> {
        let password_hash = hash_password(password)?;

        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (email, password_hash, email_verified) VALUES ($1, $2, TRUE) RETURNING *",
        )
        .bind(email)
        .bind(password_hash)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(user)
    }

    pub fn verify_password(&self, user: &User, password: &str) -> Result<bool> {
        verify_password(password, &user.password_hash)
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
