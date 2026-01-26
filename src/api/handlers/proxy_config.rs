use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::AppState;
use crate::error::Result;
use crate::gateway::config_cache::CachedRoute;
use crate::models::{JwtConfigRow, ProxyRoute, RateLimitRule};

async fn refresh_route_cache(state: &AppState) {
    if let Some(ref cache) = state.config_cache {
        if let Ok(routes) = state.proxy_config_service.list_routes().await {
            let cached: Vec<CachedRoute> = routes
                .into_iter()
                .filter(|r| r.enabled)
                .map(|r| CachedRoute {
                    path_prefix: r.path_prefix,
                    upstream_address: r.upstream_address,
                    require_auth: r.require_auth,
                    strip_prefix: r.strip_prefix,
                })
                .collect();
            cache.update_routes(cached);
            tracing::info!("Route cache refreshed");
        }
    }
}

#[derive(Deserialize)]
pub struct CreateRouteRequest {
    pub path_prefix: String,
    pub upstream_address: String,
    pub require_auth: bool,
    pub strip_prefix: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateRouteRequest {
    pub path_prefix: String,
    pub upstream_address: String,
    pub require_auth: bool,
    pub strip_prefix: Option<String>,
    pub enabled: bool,
}

pub async fn list_routes(State(state): State<AppState>) -> Result<Json<Vec<ProxyRoute>>> {
    let routes = state.proxy_config_service.list_routes().await?;
    Ok(Json(routes))
}

pub async fn create_route(
    State(state): State<AppState>,
    Json(req): Json<CreateRouteRequest>,
) -> Result<Json<ProxyRoute>> {
    let route = state
        .proxy_config_service
        .create_route(
            &req.path_prefix,
            &req.upstream_address,
            req.require_auth,
            req.strip_prefix.as_deref(),
        )
        .await?;
    refresh_route_cache(&state).await;
    Ok(Json(route))
}

pub async fn update_route(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateRouteRequest>,
) -> Result<Json<ProxyRoute>> {
    let route = state
        .proxy_config_service
        .update_route(
            id,
            &req.path_prefix,
            &req.upstream_address,
            req.require_auth,
            req.strip_prefix.as_deref(),
            req.enabled,
        )
        .await?;
    refresh_route_cache(&state).await;
    Ok(Json(route))
}

pub async fn delete_route(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>> {
    state.proxy_config_service.delete_route(id).await?;
    refresh_route_cache(&state).await;
    Ok(Json(()))
}

#[derive(Deserialize)]
pub struct CreateRateLimitRequest {
    pub name: String,
    pub path_pattern: String,
    pub limit_by: String,
    pub max_requests: i32,
    pub window_secs: i32,
}

#[derive(Deserialize)]
pub struct UpdateRateLimitRequest {
    pub name: String,
    pub path_pattern: String,
    pub limit_by: String,
    pub max_requests: i32,
    pub window_secs: i32,
    pub enabled: bool,
}

pub async fn list_rate_limits(State(state): State<AppState>) -> Result<Json<Vec<RateLimitRule>>> {
    let rules = state.proxy_config_service.list_rate_limits().await?;
    Ok(Json(rules))
}

pub async fn create_rate_limit(
    State(state): State<AppState>,
    Json(req): Json<CreateRateLimitRequest>,
) -> Result<Json<RateLimitRule>> {
    let rule = state
        .proxy_config_service
        .create_rate_limit(&req.name, &req.path_pattern, &req.limit_by, req.max_requests, req.window_secs)
        .await?;
    Ok(Json(rule))
}

pub async fn update_rate_limit(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateRateLimitRequest>,
) -> Result<Json<RateLimitRule>> {
    let rule = state
        .proxy_config_service
        .update_rate_limit(id, &req.name, &req.path_pattern, &req.limit_by, req.max_requests, req.window_secs, req.enabled)
        .await?;
    Ok(Json(rule))
}

pub async fn delete_rate_limit(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>> {
    state.proxy_config_service.delete_rate_limit(id).await?;
    Ok(Json(()))
}

#[derive(Deserialize)]
pub struct UpdateJwtConfigRequest {
    pub access_token_ttl_secs: i32,
    pub refresh_token_ttl_secs: i32,
    pub auto_refresh_threshold_secs: i32,
}

pub async fn get_jwt_config(State(state): State<AppState>) -> Result<Json<JwtConfigRow>> {
    let config = state.proxy_config_service.get_jwt_config().await?;
    Ok(Json(config))
}

pub async fn update_jwt_config(
    State(state): State<AppState>,
    Json(req): Json<UpdateJwtConfigRequest>,
) -> Result<Json<JwtConfigRow>> {
    let config = state
        .proxy_config_service
        .update_jwt_config(req.access_token_ttl_secs, req.refresh_token_ttl_secs, req.auto_refresh_threshold_secs)
        .await?;
    Ok(Json(config))
}
