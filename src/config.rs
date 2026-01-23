use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub upstream: UpstreamConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    #[serde(default)]
    pub routing: RoutesConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub gateway_port: u16,
    pub api_port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamConfig {
    pub default_upstream: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouteConfig {
    pub path: String,
    pub upstream: String,
    #[serde(default)]
    pub auth: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RoutesConfig {
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub access_token_ttl: i64,
    pub refresh_token_ttl: i64,
    pub auto_refresh_threshold: i64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            access_token_ttl: 86400,
            refresh_token_ttl: 604800,
            auto_refresh_threshold: 3600,
        }
    }
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Arc<Self>> {
        dotenvy::dotenv().ok();

        let config = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(config::File::with_name("config/local").required(false))
            .add_source(config::Environment::with_prefix("ARC_AUTH").separator("__"))
            .build()?;

        let app_config: AppConfig = config.try_deserialize()?;
        Ok(Arc::new(app_config))
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            gateway_port: 8080,
            api_port: 3001,
        }
    }
}
