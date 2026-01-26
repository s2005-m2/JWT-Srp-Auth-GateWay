use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

use crate::api::AppState;
use crate::error::{AppError, Result};
use crate::models::{User, UserInfo, VerificationCode};

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub email: String,
    pub code: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user: UserInfo,
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn verify(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<AuthResponse>> {
    validate_password(&req.password)?;

    let mut tx = state.db_pool.begin().await?;

    let record = sqlx::query_as::<_, VerificationCode>(
        "SELECT * FROM verification_codes 
         WHERE email = $1 AND code = $2 AND code_type = $3 
         AND expires_at > NOW() AND used = FALSE
         ORDER BY created_at DESC LIMIT 1
         FOR UPDATE SKIP LOCKED"
    )
    .bind(&req.email)
    .bind(&req.code)
    .bind("register")
    .fetch_optional(&mut *tx)
    .await?;

    let vc = match record {
        Some(vc) => vc,
        None => return Err(AppError::InvalidCode),
    };

    sqlx::query("UPDATE verification_codes SET used = TRUE WHERE id = $1")
        .bind(vc.id)
        .execute(&mut *tx)
        .await?;

    let user = create_user_in_tx(&mut tx, &req.email, &req.password).await?;
    
    tx.commit().await?;

    let tokens = generate_tokens(&state, &user).await?;

    Ok(Json(AuthResponse {
        user: UserInfo {
            id: user.id.to_string(),
            email: user.email,
        },
        access_token: tokens.0,
        refresh_token: tokens.1,
    }))
}

fn validate_password(password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(AppError::WeakPassword);
    }
    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());

    if !has_upper || !has_lower || !has_digit {
        return Err(AppError::WeakPassword);
    }
    Ok(())
}

async fn generate_tokens(state: &AppState, user: &User) -> Result<(String, String)> {
    let access = state.token_service.generate_access_token(user.id, &user.email).await?;
    let refresh = state.token_service.generate_refresh_token(user.id).await?;
    Ok((access, refresh))
}

async fn create_user_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    email: &str,
    password: &str,
) -> Result<User> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to hash password")))?
        .to_string();

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash, email_verified) VALUES ($1, $2, TRUE) RETURNING *"
    )
    .bind(email)
    .bind(&password_hash)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("users_email_key") {
                return AppError::EmailExists;
            }
        }
        AppError::Database(e)
    })?;

    Ok(user)
}
