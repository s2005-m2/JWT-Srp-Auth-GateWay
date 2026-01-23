use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::Result;
use crate::models::{JwtConfigRow, ProxyRoute, ProxyUpstream, RateLimitRule};

pub struct ProxyConfigService {
    pool: Arc<PgPool>,
}

impl ProxyConfigService {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn list_upstreams(&self) -> Result<Vec<ProxyUpstream>> {
        let upstreams = sqlx::query_as::<_, ProxyUpstream>(
            "SELECT * FROM proxy_upstreams ORDER BY name"
        )
        .fetch_all(self.pool.as_ref())
        .await?;
        Ok(upstreams)
    }

    pub async fn get_upstream(&self, id: Uuid) -> Result<Option<ProxyUpstream>> {
        let upstream = sqlx::query_as::<_, ProxyUpstream>(
            "SELECT * FROM proxy_upstreams WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(self.pool.as_ref())
        .await?;
        Ok(upstream)
    }

    pub async fn create_upstream(&self, name: &str, address: &str, health_check_path: Option<&str>) -> Result<ProxyUpstream> {
        let upstream = sqlx::query_as::<_, ProxyUpstream>(
            "INSERT INTO proxy_upstreams (name, address, health_check_path) VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(name)
        .bind(address)
        .bind(health_check_path)
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(upstream)
    }

    pub async fn update_upstream(&self, id: Uuid, name: &str, address: &str, health_check_path: Option<&str>, enabled: bool) -> Result<ProxyUpstream> {
        let upstream = sqlx::query_as::<_, ProxyUpstream>(
            "UPDATE proxy_upstreams SET name = $2, address = $3, health_check_path = $4, enabled = $5, updated_at = NOW() WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(name)
        .bind(address)
        .bind(health_check_path)
        .bind(enabled)
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(upstream)
    }

    pub async fn delete_upstream(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM proxy_upstreams WHERE id = $1")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }

    pub async fn list_routes(&self) -> Result<Vec<ProxyRoute>> {
        let routes = sqlx::query_as::<_, ProxyRoute>(
            "SELECT * FROM proxy_routes ORDER BY priority DESC"
        )
        .fetch_all(self.pool.as_ref())
        .await?;
        Ok(routes)
    }

    pub async fn create_route(
        &self,
        path_prefix: &str,
        upstream_id: Uuid,
        strip_prefix: bool,
        require_auth: bool,
        priority: i32,
    ) -> Result<ProxyRoute> {
        let route = sqlx::query_as::<_, ProxyRoute>(
            "INSERT INTO proxy_routes (path_prefix, upstream_id, strip_prefix, require_auth, priority) 
             VALUES ($1, $2, $3, $4, $5) RETURNING *"
        )
        .bind(path_prefix)
        .bind(upstream_id)
        .bind(strip_prefix)
        .bind(require_auth)
        .bind(priority)
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(route)
    }

    pub async fn update_route(
        &self,
        id: Uuid,
        path_prefix: &str,
        upstream_id: Uuid,
        strip_prefix: bool,
        require_auth: bool,
        priority: i32,
        enabled: bool,
    ) -> Result<ProxyRoute> {
        let route = sqlx::query_as::<_, ProxyRoute>(
            "UPDATE proxy_routes SET path_prefix = $2, upstream_id = $3, strip_prefix = $4, 
             require_auth = $5, priority = $6, enabled = $7, updated_at = NOW() 
             WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(path_prefix)
        .bind(upstream_id)
        .bind(strip_prefix)
        .bind(require_auth)
        .bind(priority)
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
        let rules = sqlx::query_as::<_, RateLimitRule>(
            "SELECT * FROM rate_limit_rules ORDER BY name"
        )
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
             WHERE id = $1 RETURNING *"
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
        let config = sqlx::query_as::<_, JwtConfigRow>(
            "SELECT * FROM jwt_config WHERE id = 1"
        )
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
             WHERE id = 1 RETURNING *"
        )
        .bind(access_token_ttl_secs)
        .bind(refresh_token_ttl_secs)
        .bind(auto_refresh_threshold_secs)
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(config)
    }
}
