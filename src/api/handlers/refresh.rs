use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::error::{AppError, Result};

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>> {
    let claims = state
        .token_service
        .validate_access_token(&req.refresh_token)
        .map_err(|_| AppError::InvalidToken)?;

    let access = state
        .token_service
        .generate_access_token(claims.sub, &claims.email)?;

    Ok(Json(RefreshResponse { access_token: access }))
}
