use rand::Rng;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::Result;
use crate::models::{SmtpConfig, SystemConfig};

pub struct SystemConfigService {
    pool: Arc<PgPool>,
    cache: Arc<RwLock<Option<SystemConfig>>>,
}

impl SystemConfigService {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        let exists: Option<(i32,)> = sqlx::query_as("SELECT id FROM system_config WHERE id = 1")
            .fetch_optional(self.pool.as_ref())
            .await?;

        if exists.is_none() {
            let jwt_secret = generate_jwt_secret();
            sqlx::query("INSERT INTO system_config (id, jwt_secret) VALUES (1, $1)")
                .bind(&jwt_secret)
                .execute(self.pool.as_ref())
                .await?;
            tracing::info!("System config initialized with new JWT secret");
        }

        self.reload_cache().await?;
        Ok(())
    }

    async fn reload_cache(&self) -> Result<()> {
        let config = sqlx::query_as::<_, SystemConfig>("SELECT * FROM system_config WHERE id = 1")
            .fetch_one(self.pool.as_ref())
            .await?;

        let mut cache = self.cache.write().await;
        *cache = Some(config);
        Ok(())
    }

    pub async fn invalidate_cache(&self) {
        let mut cache = self.cache.write().await;
        *cache = None;
    }

    async fn get_config(&self) -> Result<SystemConfig> {
        {
            let cache = self.cache.read().await;
            if let Some(ref config) = *cache {
                return Ok(config.clone());
            }
        }
        self.reload_cache().await?;
        let cache = self.cache.read().await;
        cache.as_ref().cloned().ok_or_else(|| {
            crate::error::AppError::Internal(anyhow::anyhow!("Failed to load system config"))
        })
    }

    pub async fn get_smtp_config(&self) -> Result<SmtpConfig> {
        let config = self.get_config().await?;
        Ok(SmtpConfig {
            smtp_host: config.smtp_host,
            smtp_port: config.smtp_port,
            smtp_user: config.smtp_user,
            smtp_pass: config.smtp_pass,
            from_email: config.from_email,
            from_name: config.from_name,
        })
    }

    pub async fn update_smtp_config(&self, smtp: &SmtpConfig) -> Result<SmtpConfig> {
        sqlx::query(
            "UPDATE system_config SET 
                smtp_host = $1, smtp_port = $2, smtp_user = $3, 
                smtp_pass = $4, from_email = $5, from_name = $6,
                updated_at = NOW()
             WHERE id = 1",
        )
        .bind(&smtp.smtp_host)
        .bind(smtp.smtp_port)
        .bind(&smtp.smtp_user)
        .bind(&smtp.smtp_pass)
        .bind(&smtp.from_email)
        .bind(&smtp.from_name)
        .execute(self.pool.as_ref())
        .await?;

        self.invalidate_cache().await;
        self.get_smtp_config().await
    }

    pub async fn get_jwt_secret(&self) -> Result<String> {
        let config = self.get_config().await?;
        Ok(config.jwt_secret)
    }

    pub async fn get_jwt_secret_updated_at(&self) -> Result<chrono::DateTime<chrono::Utc>> {
        let config = self.get_config().await?;
        Ok(config.jwt_secret_updated_at)
    }

    pub async fn rotate_jwt_secret(&self) -> Result<chrono::DateTime<chrono::Utc>> {
        let new_secret = generate_jwt_secret();
        sqlx::query(
            "UPDATE system_config SET 
                jwt_secret = $1, jwt_secret_updated_at = NOW(), updated_at = NOW()
             WHERE id = 1",
        )
        .bind(&new_secret)
        .execute(self.pool.as_ref())
        .await?;

        self.invalidate_cache().await;
        tracing::warn!("JWT secret rotated - all existing tokens are now invalid");
        self.get_jwt_secret_updated_at().await
    }

    pub async fn should_auto_rotate(&self) -> Result<bool> {
        let updated_at = self.get_jwt_secret_updated_at().await?;
        let now = chrono::Utc::now();
        let days_since_update = (now - updated_at).num_days();
        Ok(days_since_update >= 30)
    }
}

fn generate_jwt_secret() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
