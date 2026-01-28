use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::net::SocketAddr;

use crate::api::middleware::RateLimiter;
use crate::api::AppState;
use crate::models::ApiKeyPermissions;

#[derive(Serialize)]
struct ApiKeyError {
    error: ApiKeyErrorBody,
}

#[derive(Serialize)]
struct ApiKeyErrorBody {
    code: &'static str,
    message: &'static str,
}

pub async fn api_key_auth_middleware(
    State(state): State<AppState>,
    rate_limiter: RateLimiter,
    mut request: Request,
    next: Next,
) -> Response {
    let client_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok());

    let api_key = match api_key {
        Some(k) if k.len() == 64 => k,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiKeyError {
                    error: ApiKeyErrorBody {
                        code: "MISSING_API_KEY",
                        message: "X-API-Key header required",
                    },
                }),
            )
                .into_response();
        }
    };

    let rate_key = format!("apikey:{}", &api_key[..8]);
    if !rate_limiter.check(&rate_key) {
        tracing::warn!(key_prefix = &api_key[..8], ip = %client_ip, "API key rate limited");
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiKeyError {
                error: ApiKeyErrorBody {
                    code: "RATE_LIMITED",
                    message: "Too many requests",
                },
            }),
        )
            .into_response();
    }

    let key_record = match state.api_key_service.find_by_key(api_key).await {
        Ok(Some(k)) => k,
        Ok(None) => {
            tracing::warn!(key_prefix = &api_key[..8], ip = %client_ip, "Invalid API key");
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiKeyError {
                    error: ApiKeyErrorBody {
                        code: "INVALID_API_KEY",
                        message: "Invalid API key",
                    },
                }),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to validate API key");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiKeyError {
                    error: ApiKeyErrorBody {
                        code: "INTERNAL_ERROR",
                        message: "Internal server error",
                    },
                }),
            )
                .into_response();
        }
    };

    request.extensions_mut().insert(key_record.admin_id);
    request.extensions_mut().insert(ApiKeyPermissions::new(&key_record.permissions));

    next.run(request).await
}
