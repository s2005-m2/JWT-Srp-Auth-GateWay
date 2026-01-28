use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::api::AppState;
use crate::error::{AppError, Result};
use crate::models::UserInfo;

const MAX_VERIFICATION_ATTEMPTS: i32 = 5;

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub email: String,
    pub code: String,
    pub salt: String,
    pub verifier: String,
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
    let mut tx = state.db_pool.begin().await?;

    let record: Option<(uuid::Uuid, i32)> = sqlx::query_as(
        "SELECT id, attempts FROM verification_codes 
         WHERE email = $1 AND code_type = $2 
         AND expires_at > NOW() AND used = FALSE
         ORDER BY created_at DESC LIMIT 1
         FOR UPDATE SKIP LOCKED"
    )
    .bind(&req.email)
    .bind("register")
    .fetch_optional(&mut *tx)
    .await?;

    let (code_id, attempts) = match record {
        Some(r) => r,
        None => return Err(AppError::InvalidCode),
    };

    if attempts >= MAX_VERIFICATION_ATTEMPTS {
        warn!(email = %req.email, "Verification code exhausted (max attempts reached)");
        return Err(AppError::InvalidCode);
    }

    sqlx::query("UPDATE verification_codes SET attempts = attempts + 1 WHERE id = $1")
        .bind(code_id)
        .execute(&mut *tx)
        .await?;

    let valid: Option<(bool,)> = sqlx::query_as(
        "SELECT used FROM verification_codes WHERE id = $1 AND code = $2"
    )
    .bind(code_id)
    .bind(&req.code)
    .fetch_optional(&mut *tx)
    .await?;

    if valid.is_none() {
        tx.commit().await?;
        return Err(AppError::InvalidCode);
    }

    sqlx::query("UPDATE verification_codes SET used = TRUE WHERE id = $1")
        .bind(code_id)
        .execute(&mut *tx)
        .await?;

    let user_id = create_user_srp(&mut tx, &req.email, &req.salt, &req.verifier).await?;
    
    tx.commit().await?;

    let access = state.token_service.generate_access_token(user_id, &req.email).await?;
    let refresh = state.token_service.generate_refresh_token(user_id).await?;

    Ok(Json(AuthResponse {
        user: UserInfo {
            id: user_id.to_string(),
            email: req.email,
        },
        access_token: access,
        refresh_token: refresh,
    }))
}

async fn create_user_srp(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    email: &str,
    salt: &str,
    verifier: &str,
) -> Result<uuid::Uuid> {
    let (id,): (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO users (email, email_verified, srp_salt, srp_verifier) 
         VALUES ($1, TRUE, $2, $3) RETURNING id"
    )
    .bind(email)
    .bind(salt)
    .bind(verifier)
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

    Ok(id)
}
