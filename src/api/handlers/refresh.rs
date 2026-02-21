use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::error::Result;

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub should_refresh: bool,
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>> {
    let claims = state
        .token_service
        .validate_refresh_token(&req.refresh_token)
        .await?;

    let user = state
        .user_service
        .find_by_id(claims.sub)
        .await?
        .ok_or(crate::error::AppError::InvalidToken)?;

    state
        .token_service
        .revoke_refresh_token(&req.refresh_token)
        .await?;

    let access = state
        .token_service
        .generate_access_token(user.id, &user.email)
        .await?;
    let refresh = state.token_service.generate_refresh_token(user.id).await?;

    let access_claims = state.token_service.validate_access_token(&access).await?;
    let should_refresh = state.token_service.should_refresh(&access_claims);

    Ok(Json(RefreshResponse {
        access_token: access,
        refresh_token: refresh,
        should_refresh,
    }))
}
