use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::api::AppState;
use crate::error::{AppError, Result};
use crate::models::ApiKey;

#[derive(Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: Vec<String>,
}

#[derive(Serialize)]
pub struct CreateApiKeyResponse {
    pub api_key: ApiKey,
    pub raw_key: String,
}

#[derive(Serialize)]
pub struct ApiKeyListResponse {
    pub api_keys: Vec<ApiKey>,
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    axum::Extension(admin_id): axum::Extension<Uuid>,
) -> Result<Json<ApiKeyListResponse>> {
    let api_keys = state.api_key_service.list_by_admin(admin_id).await?;
    Ok(Json(ApiKeyListResponse { api_keys }))
}

pub async fn create_api_key(
    State(state): State<AppState>,
    axum::Extension(admin_id): axum::Extension<Uuid>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>> {
    if req.name.is_empty() || req.name.len() > 255 {
        return Err(AppError::InvalidRequest("Name must be 1-255 characters".into()));
    }

    let (api_key, raw_key) = state
        .api_key_service
        .create(admin_id, &req.name, req.permissions)
        .await?;

    info!(admin_id = %admin_id, key_id = %api_key.id, "API key created");

    Ok(Json(CreateApiKeyResponse { api_key, raw_key }))
}

pub async fn delete_api_key(
    State(state): State<AppState>,
    axum::Extension(admin_id): axum::Extension<Uuid>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let deleted = state.api_key_service.delete(id, admin_id).await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    info!(admin_id = %admin_id, key_id = %id, "API key deleted");

    Ok(Json(serde_json::json!({ "success": true })))
}
