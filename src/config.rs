use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub upstream: UpstreamConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub email: EmailConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub gateway_port: u16,
    pub api_port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamConfig {
    pub arc_generater: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_ttl: i64,
    pub refresh_token_ttl: i64,
    pub auto_refresh_threshold: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub from_email: String,
    pub from_name: String,
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

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: String::new(),
            access_token_ttl: 86400,
            refresh_token_ttl: 604800,
            auto_refresh_threshold: 3600,
        }
    }
}
