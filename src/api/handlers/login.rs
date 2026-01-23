use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::api::AppState;
use crate::error::{AppError, Result};
use crate::models::UserInfo;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user: UserInfo,
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    info!(email = %req.email, "Login attempt");

    let user = match state.user_service.find_by_email(&req.email).await? {
        Some(u) => u,
        None => {
            warn!(email = %req.email, "Login failed: user not found");
            return Err(AppError::InvalidCredentials);
        }
    };

    let valid = state.user_service.verify_password(&user, &req.password)?;
    if !valid {
        warn!(email = %req.email, user_id = %user.id, "Login failed: invalid password");
        return Err(AppError::InvalidCredentials);
    }

    let access = state.token_service.generate_access_token(user.id, &user.email)?;
    let refresh = state.token_service.generate_refresh_token(user.id).await?;

    info!(email = %req.email, user_id = %user.id, "Login successful");

    Ok(Json(LoginResponse {
        user: UserInfo {
            id: user.id.to_string(),
            email: user.email,
        },
        access_token: access,
        refresh_token: refresh,
    }))
}
