use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::api::AppState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub message: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>> {
    info!(email = %req.email, "Registration attempt");

    if !is_valid_email(&req.email) {
        warn!(email = %req.email, "Registration failed: invalid email format");
        return Err(AppError::InvalidEmail);
    }

    if state.user_service.find_by_email(&req.email).await?.is_some() {
        warn!(email = %req.email, "Registration failed: email already exists");
        return Err(AppError::EmailExists);
    }

    let code = generate_code();
    save_verification_code(&state, &req.email, &code, "register").await?;
    state.email_service.send_verification_code(&req.email, &code).await?;

    info!(email = %req.email, "Registration verification code sent");

    Ok(Json(RegisterResponse {
        message: "Verification code sent".to_string(),
    }))
}

fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() >= 5
}

fn generate_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..1000000))
}

async fn save_verification_code(
    state: &AppState,
    email: &str,
    code: &str,
    code_type: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO verification_codes (email, code, code_type, expires_at) 
         VALUES ($1, $2, $3, NOW() + INTERVAL '10 minutes')"
    )
    .bind(email)
    .bind(code)
    .bind(code_type)
    .execute(state.db_pool.as_ref())
    .await?;
    Ok(())
}
