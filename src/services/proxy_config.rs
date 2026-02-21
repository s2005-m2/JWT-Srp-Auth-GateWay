use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::Result;
use crate::models::{JwtConfigRow, ProxyRoute, RateLimitRule};

pub struct ProxyConfigService {
    pool: Arc<PgPool>,
}

impl ProxyConfigService {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn list_routes(&self) -> Result<Vec<ProxyRoute>> {
        let routes =
            sqlx::query_as::<_, ProxyRoute>("SELECT * FROM proxy_routes ORDER BY path_prefix")
                .fetch_all(self.pool.as_ref())
                .await?;
        Ok(routes)
    }

    pub async fn create_route(
        &self,
        path_prefix: &str,
        upstream_address: &str,
        require_auth: bool,
        strip_prefix: Option<&str>,
    ) -> Result<ProxyRoute> {
        let route = sqlx::query_as::<_, ProxyRoute>(
            "INSERT INTO proxy_routes (path_prefix, upstream_address, require_auth, strip_prefix) 
             VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(path_prefix)
        .bind(upstream_address)
        .bind(require_auth)
        .bind(strip_prefix)
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(route)
    }

    pub async fn update_route(
        &self,
        id: Uuid,
        path_prefix: &str,
        upstream_address: &str,
        require_auth: bool,
        strip_prefix: Option<&str>,
        enabled: bool,
    ) -> Result<ProxyRoute> {
        let route = sqlx::query_as::<_, ProxyRoute>(
            "UPDATE proxy_routes SET path_prefix = $2, upstream_address = $3, 
             require_auth = $4, strip_prefix = $5, enabled = $6, updated_at = NOW() 
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(path_prefix)
        .bind(upstream_address)
        .bind(require_auth)
        .bind(strip_prefix)
        .bind(enabled)
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(route)
    }

    pub async fn delete_route(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM proxy_routes WHERE id = $1")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }

    pub async fn list_rate_limits(&self) -> Result<Vec<RateLimitRule>> {
        let rules =
            sqlx::query_as::<_, RateLimitRule>("SELECT * FROM rate_limit_rules ORDER BY name")
                .fetch_all(self.pool.as_ref())
                .await?;
        Ok(rules)
    }

    pub async fn create_rate_limit(
        &self,
        name: &str,
        path_pattern: &str,
        limit_by: &str,
        max_requests: i32,
        window_secs: i32,
    ) -> Result<RateLimitRule> {
        let rule = sqlx::query_as::<_, RateLimitRule>(
            "INSERT INTO rate_limit_rules (name, path_pattern, limit_by, max_requests, window_secs) 
             VALUES ($1, $2, $3, $4, $5) RETURNING *"
        )
        .bind(name)
        .bind(path_pattern)
        .bind(limit_by)
        .bind(max_requests)
        .bind(window_secs)
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(rule)
    }

    pub async fn update_rate_limit(
        &self,
        id: Uuid,
        name: &str,
        path_pattern: &str,
        limit_by: &str,
        max_requests: i32,
        window_secs: i32,
        enabled: bool,
    ) -> Result<RateLimitRule> {
        let rule = sqlx::query_as::<_, RateLimitRule>(
            "UPDATE rate_limit_rules SET name = $2, path_pattern = $3, limit_by = $4, 
             max_requests = $5, window_secs = $6, enabled = $7, updated_at = NOW() 
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(name)
        .bind(path_pattern)
        .bind(limit_by)
        .bind(max_requests)
        .bind(window_secs)
        .bind(enabled)
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(rule)
    }

    pub async fn delete_rate_limit(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM rate_limit_rules WHERE id = $1")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }

    pub async fn get_jwt_config(&self) -> Result<JwtConfigRow> {
        let config = sqlx::query_as::<_, JwtConfigRow>("SELECT * FROM jwt_config WHERE id = 1")
            .fetch_one(self.pool.as_ref())
            .await?;
        Ok(config)
    }

    pub async fn update_jwt_config(
        &self,
        access_token_ttl_secs: i32,
        refresh_token_ttl_secs: i32,
        auto_refresh_threshold_secs: i32,
    ) -> Result<JwtConfigRow> {
        let config = sqlx::query_as::<_, JwtConfigRow>(
            "UPDATE jwt_config SET access_token_ttl_secs = $1, refresh_token_ttl_secs = $2, 
             auto_refresh_threshold_secs = $3, updated_at = NOW() 
             WHERE id = 1 RETURNING *",
        )
        .bind(access_token_ttl_secs)
        .bind(refresh_token_ttl_secs)
        .bind(auto_refresh_threshold_secs)
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(config)
    }
}
