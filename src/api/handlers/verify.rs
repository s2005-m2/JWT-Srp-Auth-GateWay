use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

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

    let valid = verify_code(&state, &req.email, &req.code, "register").await?;
    if !valid {
        return Err(AppError::InvalidCode);
    }

    let user = state.user_service.create(&req.email, &req.password).await?;
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

async fn verify_code(state: &AppState, email: &str, code: &str, code_type: &str) -> Result<bool> {
    let record = sqlx::query_as::<_, VerificationCode>(
        "SELECT * FROM verification_codes 
         WHERE email = $1 AND code = $2 AND code_type = $3 
         AND expires_at > NOW() AND used = FALSE
         ORDER BY created_at DESC LIMIT 1"
    )
    .bind(email)
    .bind(code)
    .bind(code_type)
    .fetch_optional(state.db_pool.as_ref())
    .await?;

    if let Some(vc) = record {
        tracing::debug!(
            email = %vc.email,
            code_type = %vc.code_type,
            expires_at = %vc.expires_at,
            created_at = %vc.created_at,
            "Verification code validated"
        );
        sqlx::query("UPDATE verification_codes SET used = TRUE WHERE id = $1")
            .bind(vc.id)
            .execute(state.db_pool.as_ref())
            .await?;
        return Ok(true);
    }

    Ok(false)
}

async fn generate_tokens(state: &AppState, user: &User) -> Result<(String, String)> {
    let access = state.token_service.generate_access_token(user.id, &user.email).await?;
    let refresh = state.token_service.generate_refresh_token(user.id).await?;
    Ok((access, refresh))
}
