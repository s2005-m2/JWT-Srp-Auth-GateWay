use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::error::Result;
use crate::models::{JwtSecretInfo, SmtpConfig};

#[derive(Deserialize)]
pub struct UpdateSmtpConfigRequest {
    pub from_email: String,
    pub smtp_pass: String,
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
    let (smtp_host, smtp_port) = infer_smtp_config(&req.from_email);
    
    let smtp = SmtpConfig {
        smtp_host,
        smtp_port,
        smtp_user: req.from_email.clone(),
        smtp_pass: req.smtp_pass,
        from_email: req.from_email,
        from_name: "Arc Auth".to_string(),
    };
    let config = state.system_config_service.update_smtp_config(&smtp).await?;
    Ok(Json(config))
}

fn infer_smtp_config(email: &str) -> (String, i32) {
    let domain = email.split('@').nth(1).unwrap_or("");
    
    match domain {
        "qq.com" => ("smtp.qq.com".to_string(), 465),
        "163.com" => ("smtp.163.com".to_string(), 465),
        "126.com" => ("smtp.126.com".to_string(), 465),
        "yeah.net" => ("smtp.yeah.net".to_string(), 465),
        "sina.com" => ("smtp.sina.com".to_string(), 465),
        "gmail.com" => ("smtp.gmail.com".to_string(), 587),
        "outlook.com" | "hotmail.com" => ("smtp.office365.com".to_string(), 587),
        "yahoo.com" => ("smtp.mail.yahoo.com".to_string(), 587),
        "icloud.com" => ("smtp.mail.me.com".to_string(), 587),
        _ => (format!("smtp.{}", domain), 587),
    }
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
