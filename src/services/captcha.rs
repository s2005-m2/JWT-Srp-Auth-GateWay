use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};

pub struct CaptchaService {
    db_pool: Arc<PgPool>,
}

impl CaptchaService {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }

    pub async fn generate(&self) -> Result<(String, String)> {
        let (text, png_bytes) = tokio::task::spawn_blocking(|| {
            captcha::Captcha::new()
                .add_chars(5)
                .view(220, 120)
                .as_tuple()
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("captcha generation failed")))
        })
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("spawn_blocking error: {}", e)))??;

        let image_b64 = STANDARD.encode(&png_bytes);

        let row: (Uuid,) = sqlx::query_as("INSERT INTO captchas (text) VALUES ($1) RETURNING id")
            .bind(&text)
            .fetch_one(self.db_pool.as_ref())
            .await?;

        Ok((row.0.to_string(), image_b64))
    }

    pub async fn validate(&self, captcha_id: &str, text: &str) -> Result<()> {
        let id = Uuid::parse_str(captcha_id).map_err(|_| AppError::InvalidCaptcha)?;

        let row: Option<(String,)> = sqlx::query_as(
            "UPDATE captchas SET used = true \
             WHERE id = $1 AND used = false AND expires_at > NOW() \
             RETURNING text",
        )
        .bind(id)
        .fetch_optional(self.db_pool.as_ref())
        .await?;

        let stored = row.ok_or(AppError::InvalidCaptcha)?.0;

        if stored.to_lowercase() != text.to_lowercase() {
            return Err(AppError::InvalidCaptcha);
        }

        Ok(())
    }
}
