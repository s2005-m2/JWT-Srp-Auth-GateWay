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

    #[error("Invalid captcha")]
    InvalidCaptcha,

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

    #[error("Resource not found")]
    NotFound,

    #[error("Rate limit exceeded")]
    #[allow(dead_code)]
    RateLimited,

    #[error("Access forbidden")]
    Forbidden,

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
            | Self::InvalidCaptcha
            | Self::WeakPassword
            | Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::InvalidCredentials
            | Self::InvalidToken
            | Self::TokenExpired
            | Self::TokenRevoked => StatusCode::UNAUTHORIZED,
            Self::EmailNotVerified | Self::Forbidden => StatusCode::FORBIDDEN,
            Self::EmailExists => StatusCode::CONFLICT,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal(_) | Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidEmail => "INVALID_EMAIL",
            Self::InvalidCode => "INVALID_CODE",
            Self::InvalidCaptcha => "INVALID_CAPTCHA",
            Self::WeakPassword => "WEAK_PASSWORD",
            Self::InvalidRequest(_) => "INVALID_REQUEST",
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::TokenExpired => "TOKEN_EXPIRED",
            Self::TokenRevoked => "TOKEN_REVOKED",
            Self::EmailNotVerified => "EMAIL_NOT_VERIFIED",
            Self::EmailExists => "EMAIL_EXISTS",
            Self::NotFound => "NOT_FOUND",
            Self::RateLimited => "RATE_LIMITED",
            Self::Forbidden => "FORBIDDEN",
            Self::Internal(_) | Self::Database(_) => "INTERNAL_ERROR",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        self.log_error();

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

impl AppError {
    fn log_error(&self) {
        match self {
            Self::Database(e) => {
                tracing::error!(error = %e, error_debug = ?e, "Database error occurred");
            }
            Self::Internal(e) => {
                tracing::error!(error = %e, error_debug = ?e, "Internal error occurred");
            }
            Self::InvalidCredentials => {
                tracing::warn!(
                    error_code = "INVALID_CREDENTIALS",
                    "Security: authentication failed"
                );
            }
            Self::InvalidToken => {
                tracing::warn!(
                    error_code = "INVALID_TOKEN",
                    "Security: invalid token presented"
                );
            }
            Self::TokenExpired => {
                tracing::warn!(
                    error_code = "TOKEN_EXPIRED",
                    "Security: expired token presented"
                );
            }
            Self::TokenRevoked => {
                tracing::warn!(
                    error_code = "TOKEN_REVOKED",
                    "Security: revoked token presented"
                );
            }
            Self::RateLimited => {
                tracing::warn!(error_code = "RATE_LIMITED", "Security: rate limit exceeded");
            }
            Self::InvalidEmail => {
                tracing::info!(
                    error_code = "INVALID_EMAIL",
                    "Validation: invalid email format"
                );
            }
            Self::InvalidCode => {
                tracing::info!(
                    error_code = "INVALID_CODE",
                    "Validation: invalid verification code"
                );
            }
            Self::InvalidCaptcha => {
                tracing::info!(
                    error_code = "INVALID_CAPTCHA",
                    "Validation: invalid captcha"
                );
            }
            Self::WeakPassword => {
                tracing::info!(
                    error_code = "WEAK_PASSWORD",
                    "Validation: weak password rejected"
                );
            }
            Self::EmailExists => {
                tracing::info!(
                    error_code = "EMAIL_EXISTS",
                    "Validation: duplicate email registration"
                );
            }
            Self::EmailNotVerified => {
                tracing::info!(
                    error_code = "EMAIL_NOT_VERIFIED",
                    "Validation: unverified email access"
                );
            }
            Self::InvalidRequest(msg) => {
                tracing::info!(error_code = "INVALID_REQUEST", message = %msg, "Validation: invalid request");
            }
            Self::NotFound => {
                tracing::info!(error_code = "NOT_FOUND", "Resource not found");
            }
            Self::Forbidden => {
                tracing::warn!(error_code = "FORBIDDEN", "Security: access forbidden");
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_codes_bad_request() {
        assert_eq!(
            AppError::InvalidEmail.status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(AppError::InvalidCode.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(
            AppError::WeakPassword.status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::InvalidRequest("test".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn test_status_codes_unauthorized() {
        assert_eq!(
            AppError::InvalidCredentials.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::InvalidToken.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::TokenExpired.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::TokenRevoked.status_code(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn test_status_codes_other() {
        assert_eq!(
            AppError::EmailNotVerified.status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(AppError::EmailExists.status_code(), StatusCode::CONFLICT);
        assert_eq!(
            AppError::RateLimited.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(AppError::InvalidEmail.error_code(), "INVALID_EMAIL");
        assert_eq!(
            AppError::InvalidCredentials.error_code(),
            "INVALID_CREDENTIALS"
        );
        assert_eq!(AppError::InvalidToken.error_code(), "INVALID_TOKEN");
        assert_eq!(AppError::TokenExpired.error_code(), "TOKEN_EXPIRED");
        assert_eq!(AppError::TokenRevoked.error_code(), "TOKEN_REVOKED");
        assert_eq!(AppError::RateLimited.error_code(), "RATE_LIMITED");
    }

    #[test]
    fn test_error_messages() {
        assert_eq!(AppError::InvalidEmail.to_string(), "Invalid email format");
        assert_eq!(
            AppError::InvalidCredentials.to_string(),
            "Invalid credentials"
        );
        assert_eq!(
            AppError::InvalidRequest("bad data".into()).to_string(),
            "Invalid request: bad data"
        );
    }
}
