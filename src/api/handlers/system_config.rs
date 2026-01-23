use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::error::Result;
use crate::models::{JwtSecretInfo, SmtpConfig};

#[derive(Deserialize)]
pub struct UpdateSmtpConfigRequest {
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub from_email: String,
    pub from_name: String,
}

#[derive(Deserialize)]
pub struct RotateJwtSecretRequest {
    pub confirmation: String,
}

#[derive(Serialize)]
pub struct RotateJwtSecretResponse {
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub message: String,
}

pub async fn get_smtp_config(State(state): State<AppState>) -> Result<Json<SmtpConfig>> {
    let config = state.system_config_service.get_smtp_config().await?;
    Ok(Json(config))
}

pub async fn update_smtp_config(
    State(state): State<AppState>,
    Json(req): Json<UpdateSmtpConfigRequest>,
) -> Result<Json<SmtpConfig>> {
    let smtp = SmtpConfig {
        smtp_host: req.smtp_host,
        smtp_port: req.smtp_port,
        smtp_user: req.smtp_user,
        smtp_pass: req.smtp_pass,
        from_email: req.from_email,
        from_name: req.from_name,
    };
    let config = state.system_config_service.update_smtp_config(&smtp).await?;
    Ok(Json(config))
}

pub async fn get_jwt_secret_info(State(state): State<AppState>) -> Result<Json<JwtSecretInfo>> {
    let updated_at = state.system_config_service.get_jwt_secret_updated_at().await?;
    Ok(Json(JwtSecretInfo { updated_at }))
}

pub async fn rotate_jwt_secret(
    State(state): State<AppState>,
    Json(req): Json<RotateJwtSecretRequest>,
) -> Result<Json<RotateJwtSecretResponse>> {
    if req.confirmation != "确定刷新" {
        return Err(crate::error::AppError::InvalidRequest(
            "Please type '确定刷新' to confirm".into(),
        ));
    }

    let updated_at = state.system_config_service.rotate_jwt_secret().await?;

    if let Some(ref jwt_validator) = state.jwt_validator {
        jwt_validator.refresh_secret().await.ok();
    }

    Ok(Json(RotateJwtSecretResponse {
        updated_at,
        message: "JWT secret rotated. All existing tokens are now invalid.".into(),
    }))
}
