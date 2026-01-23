use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::error::{AppError, Result};

#[derive(Deserialize)]
pub struct AdminRegisterRequest {
    pub username: String,
    pub password: String,
    pub registration_token: String,
}

#[derive(Deserialize)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AdminAuthResponse {
    pub admin: AdminInfo,
    pub access_token: String,
}

#[derive(Serialize)]
pub struct AdminInfo {
    pub id: String,
    pub username: String,
}

pub async fn admin_register(
    State(state): State<AppState>,
    Json(req): Json<AdminRegisterRequest>,
) -> Result<Json<AdminAuthResponse>> {
    if req.username.len() < 3 {
        return Err(AppError::InvalidRequest("Username must be at least 3 characters".into()));
    }
    if req.password.len() < 8 {
        return Err(AppError::WeakPassword);
    }

    let admin = state
        .admin_service
        .create_with_token(&req.username, &req.password, &req.registration_token)
        .await?;

    let token = state.admin_service.generate_admin_jwt(&admin)?;

    Ok(Json(AdminAuthResponse {
        admin: AdminInfo {
            id: admin.id.to_string(),
            username: admin.username,
        },
        access_token: token,
    }))
}

pub async fn admin_login(
    State(state): State<AppState>,
    Json(req): Json<AdminLoginRequest>,
) -> Result<Json<AdminAuthResponse>> {
    let admin = state
        .admin_service
        .find_by_username(&req.username)
        .await?
        .ok_or(AppError::InvalidCredentials)?;

    let valid = state.admin_service.verify_password(&admin, &req.password)?;
    if !valid {
        return Err(AppError::InvalidCredentials);
    }

    let token = state.admin_service.generate_admin_jwt(&admin)?;

    Ok(Json(AdminAuthResponse {
        admin: AdminInfo {
            id: admin.id.to_string(),
            username: admin.username,
        },
        access_token: token,
    }))
}
