use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::api::AppState;
use crate::error::{AppError, Result};

#[derive(Deserialize)]
pub struct RequestResetRequest {
    pub email: String,
}

#[derive(Serialize)]
pub struct RequestResetResponse {
    pub message: String,
}

pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(req): Json<RequestResetRequest>,
) -> Result<Json<RequestResetResponse>> {
    info!(email = %req.email, "Password reset requested");

    let user = state.user_service.find_by_email(&req.email).await?;

    if user.is_none() {
        return Ok(Json(RequestResetResponse {
            message: "If the email exists, a reset code has been sent".to_string(),
        }));
    }

    let code = generate_code();
    save_verification_code(&state, &req.email, &code, "password_reset").await?;
    state
        .email_service
        .send_password_reset(&req.email, &code)
        .await?;

    info!(email = %req.email, "Password reset code sent");

    Ok(Json(RequestResetResponse {
        message: "If the email exists, a reset code has been sent".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub email: String,
    pub code: String,
    pub salt: String,
    pub verifier: String,
}

#[derive(Serialize)]
pub struct ResetPasswordResponse {
    pub message: String,
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<ResetPasswordResponse>> {
    info!(email = %req.email, "Password reset attempt");

    let valid = verify_code(&state, &req.email, &req.code, "password_reset").await?;
    if !valid {
        warn!(email = %req.email, "Password reset failed: invalid code");
        return Err(AppError::InvalidCode);
    }

    let user = state
        .user_service
        .find_by_email(&req.email)
        .await?
        .ok_or(AppError::InvalidCredentials)?;

    update_srp_credentials(&state, user.id, &req.salt, &req.verifier).await?;

    mark_code_used(&state, &req.email, &req.code, "password_reset").await?;

    info!(email = %req.email, "Password reset successful");

    Ok(Json(ResetPasswordResponse {
        message: "Password has been reset successfully".to_string(),
    }))
}

fn generate_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..1000000))
}

async fn update_srp_credentials(
    state: &AppState,
    user_id: uuid::Uuid,
    salt: &str,
    verifier: &str,
) -> Result<()> {
    sqlx::query("UPDATE users SET srp_salt = $1, srp_verifier = $2 WHERE id = $3")
        .bind(salt)
        .bind(verifier)
        .bind(user_id)
        .execute(state.db_pool.as_ref())
        .await?;
    Ok(())
}

async fn save_verification_code(
    state: &AppState,
    email: &str,
    code: &str,
    code_type: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO verification_codes (email, code, code_type, expires_at) 
         VALUES ($1, $2, $3, NOW() + INTERVAL '10 minutes')",
    )
    .bind(email)
    .bind(code)
    .bind(code_type)
    .execute(state.db_pool.as_ref())
    .await?;
    Ok(())
}

const MAX_VERIFICATION_ATTEMPTS: i32 = 5;

async fn verify_code(state: &AppState, email: &str, code: &str, code_type: &str) -> Result<bool> {
    let result: Option<(uuid::Uuid, i32)> = sqlx::query_as(
        "SELECT id, attempts FROM verification_codes 
         WHERE email = $1 AND code_type = $2 
         AND expires_at > NOW() AND used = FALSE
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(email)
    .bind(code_type)
    .fetch_optional(state.db_pool.as_ref())
    .await?;

    let (code_id, attempts) = match result {
        Some(r) => r,
        None => return Ok(false),
    };

    if attempts >= MAX_VERIFICATION_ATTEMPTS {
        warn!(email = %email, "Verification code exhausted (max attempts reached)");
        return Ok(false);
    }

    sqlx::query("UPDATE verification_codes SET attempts = attempts + 1 WHERE id = $1")
        .bind(code_id)
        .execute(state.db_pool.as_ref())
        .await?;

    let valid: Option<(bool,)> = sqlx::query_as(
        "SELECT used FROM verification_codes 
         WHERE id = $1 AND code = $2",
    )
    .bind(code_id)
    .bind(code)
    .fetch_optional(state.db_pool.as_ref())
    .await?;

    Ok(valid.is_some())
}

async fn mark_code_used(state: &AppState, email: &str, code: &str, code_type: &str) -> Result<()> {
    sqlx::query(
        "UPDATE verification_codes SET used = TRUE 
         WHERE email = $1 AND code = $2 AND code_type = $3",
    )
    .bind(email)
    .bind(code)
    .bind(code_type)
    .execute(state.db_pool.as_ref())
    .await?;
    Ok(())
}
