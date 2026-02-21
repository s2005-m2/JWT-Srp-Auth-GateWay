use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::api::AppState;

#[derive(Serialize)]
struct AuthError {
    error: AuthErrorBody,
}

#[derive(Serialize)]
struct AuthErrorBody {
    code: &'static str,
    message: &'static str,
}

pub async fn admin_auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let token = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    let token = match token {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: AuthErrorBody {
                        code: "MISSING_TOKEN",
                        message: "Authorization header required",
                    },
                }),
            )
                .into_response();
        }
    };

    let claims = match state.admin_service.validate_admin_jwt(token).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Admin JWT validation failed: {}", e);
            return (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: AuthErrorBody {
                        code: "INVALID_TOKEN",
                        message: "Invalid or expired token",
                    },
                }),
            )
                .into_response();
        }
    };

    if state
        .admin_service
        .find_by_id(claims.sub)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(AuthError {
                error: AuthErrorBody {
                    code: "ADMIN_NOT_FOUND",
                    message: "Admin account not found",
                },
            }),
        )
            .into_response();
    }

    let mut request = request;
    request.extensions_mut().insert(claims.sub);

    next.run(request).await
}
