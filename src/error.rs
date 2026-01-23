use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Invalid email format")]
    InvalidEmail,

    #[error("Invalid verification code")]
    InvalidCode,

    #[error("Password does not meet requirements")]
    WeakPassword,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("Token revoked")]
    TokenRevoked,

    #[error("Email not verified")]
    EmailNotVerified,

    #[error("Email already exists")]
    EmailExists,

    #[error("Rate limit exceeded")]
    #[allow(dead_code)]
    RateLimited,

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),

    #[error("Database error")]
    Database(#[from] sqlx::Error),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorBody,
    request_id: Option<String>,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidEmail
            | Self::InvalidCode
            | Self::WeakPassword
            | Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::InvalidCredentials
            | Self::InvalidToken
            | Self::TokenExpired
            | Self::TokenRevoked => StatusCode::UNAUTHORIZED,
            Self::EmailNotVerified => StatusCode::FORBIDDEN,
            Self::EmailExists => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal(_) | Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidEmail => "INVALID_EMAIL",
            Self::InvalidCode => "INVALID_CODE",
            Self::WeakPassword => "WEAK_PASSWORD",
            Self::InvalidRequest(_) => "INVALID_REQUEST",
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::TokenExpired => "TOKEN_EXPIRED",
            Self::TokenRevoked => "TOKEN_REVOKED",
            Self::EmailNotVerified => "EMAIL_NOT_VERIFIED",
            Self::EmailExists => "EMAIL_EXISTS",
            Self::RateLimited => "RATE_LIMITED",
            Self::Internal(_) | Self::Database(_) => "INTERNAL_ERROR",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            Self::Database(e) => {
                tracing::error!(error = %e, error_debug = ?e, "Database error occurred");
            }
            Self::Internal(e) => {
                tracing::error!(error = %e, error_debug = ?e, "Internal error occurred");
            }
            _ => {}
        }

        let status = self.status_code();
        let body = ErrorResponse {
            error: ErrorBody {
                code: self.error_code(),
                message: self.to_string(),
            },
            request_id: None,
        };
        (status, Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
