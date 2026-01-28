use axum::{extract::State, Extension, Json};

use crate::api::AppState;
use crate::error::{AppError, Result};
use crate::models::{ApiKeyPermissions, ProxyRoute};

use super::stats::{StatsResponse, UserListResponse};

pub async fn external_stats(
    Extension(perms): Extension<ApiKeyPermissions>,
    State(state): State<AppState>,
) -> Result<Json<StatsResponse>> {
    if !perms.has("stats:read") {
        return Err(AppError::Forbidden);
    }
    super::stats::get_stats(State(state)).await
}

pub async fn external_users(
    Extension(perms): Extension<ApiKeyPermissions>,
    State(state): State<AppState>,
) -> Result<Json<UserListResponse>> {
    if !perms.has("users:read") {
        return Err(AppError::Forbidden);
    }
    super::stats::get_users(State(state)).await
}

pub async fn external_routes(
    Extension(perms): Extension<ApiKeyPermissions>,
    State(state): State<AppState>,
) -> Result<Json<Vec<ProxyRoute>>> {
    if !perms.has("routes:read") {
        return Err(AppError::Forbidden);
    }
    super::proxy_config::list_routes(State(state)).await
}
