use axum::{extract::State, Json};
use serde::Serialize;

use crate::api::AppState;
use crate::error::{AppError, Result};

#[derive(Serialize)]
pub struct CaptchaResponse {
    pub captcha_id: String,
    pub image: String,
}

pub async fn get_captcha(State(state): State<AppState>) -> Result<Json<CaptchaResponse>> {
    if !state.captcha_enabled {
        return Err(AppError::NotFound);
    }
    let (captcha_id, image) = state.captcha_service.generate().await?;
    Ok(Json(CaptchaResponse { captcha_id, image }))
}
