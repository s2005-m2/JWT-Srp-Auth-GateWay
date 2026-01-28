use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::error::Result;
use crate::models::UserInfo;

#[derive(Deserialize)]
pub struct SrpInitRequest {
    pub email: String,
    pub client_public: String,
}

#[derive(Serialize)]
pub struct SrpInitResponse {
    pub session_id: String,
    pub salt: String,
    pub server_public: String,
}

#[derive(Deserialize)]
pub struct SrpVerifyRequest {
    pub session_id: String,
    pub client_proof: String,
}

#[derive(Serialize)]
pub struct SrpVerifyResponse {
    pub user: UserInfo,
    pub server_proof: String,
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn srp_init(
    State(state): State<AppState>,
    Json(req): Json<SrpInitRequest>,
) -> Result<Json<SrpInitResponse>> {
    let session = state
        .srp_service
        .init_login(&req.email, &req.client_public)
        .await?;

    Ok(Json(SrpInitResponse {
        session_id: session.session_id.to_string(),
        salt: session.salt,
        server_public: session.server_public,
    }))
}

pub async fn srp_verify(
    State(state): State<AppState>,
    Json(req): Json<SrpVerifyRequest>,
) -> Result<Json<SrpVerifyResponse>> {
    let session_id = req.session_id.parse::<Uuid>()
        .map_err(|_| crate::error::AppError::InvalidRequest("Invalid session_id".into()))?;

    let (user_id, email, server_proof) = state
        .srp_service
        .verify_login(session_id, &req.client_proof)
        .await?;

    let access = state.token_service.generate_access_token(user_id, &email).await?;
    let refresh = state.token_service.generate_refresh_token(user_id).await?;

    Ok(Json(SrpVerifyResponse {
        user: UserInfo {
            id: user_id.to_string(),
            email,
        },
        server_proof,
        access_token: access,
        refresh_token: refresh,
    }))
}
