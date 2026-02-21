use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::Result;
use crate::models::ApiKey;

pub struct ApiKeyService {
    db_pool: Arc<PgPool>,
}

impl ApiKeyService {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }

    pub fn generate_key() -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    }

    pub fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub async fn create(
        &self,
        admin_id: Uuid,
        name: &str,
        permissions: Vec<String>,
    ) -> Result<(ApiKey, String)> {
        let raw_key = Self::generate_key();
        let key_hash = Self::hash_key(&raw_key);
        let key_prefix = &raw_key[..8];

        let api_key = sqlx::query_as::<_, ApiKey>(
            r#"
            INSERT INTO api_keys (admin_id, name, key_hash, key_prefix, permissions)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(admin_id)
        .bind(name)
        .bind(&key_hash)
        .bind(key_prefix)
        .bind(serde_json::json!(permissions))
        .fetch_one(self.db_pool.as_ref())
        .await?;

        Ok((api_key, raw_key))
    }

    pub async fn list_by_admin(&self, admin_id: Uuid) -> Result<Vec<ApiKey>> {
        let keys = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE admin_id = $1 ORDER BY created_at DESC",
        )
        .bind(admin_id)
        .fetch_all(self.db_pool.as_ref())
        .await?;

        Ok(keys)
    }

    pub async fn find_by_key(&self, raw_key: &str) -> Result<Option<ApiKey>> {
        let key_hash = Self::hash_key(raw_key);

        let key = sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE key_hash = $1")
            .bind(&key_hash)
            .fetch_optional(self.db_pool.as_ref())
            .await?;

        Ok(key)
    }

    pub async fn delete(&self, id: Uuid, admin_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM api_keys WHERE id = $1 AND admin_id = $2")
            .bind(id)
            .bind(admin_id)
            .execute(self.db_pool.as_ref())
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
