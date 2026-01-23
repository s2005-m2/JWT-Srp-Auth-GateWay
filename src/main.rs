use std::sync::{atomic::AtomicU64, Arc};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod db;
mod error;
mod gateway;
mod models;
mod services;

use api::AppState;
use config::AppConfig;
use gateway::{JwtValidator, ProxyConfigCache};
use gateway::config_cache::CachedRoute;
use services::{AdminService, EmailService, ProxyConfigService, TokenService, UserService};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();

    let config = AppConfig::load()?;
    let db_pool = db::create_pool(&config.database).await?;

    tracing::info!("Running database migrations...");
    db::run_migrations(&db_pool).await?;

    let jwt_validator = Arc::new(JwtValidator::new(&config.jwt));
    let user_service = Arc::new(UserService::new(db_pool.clone()));
    let token_service = Arc::new(TokenService::new(db_pool.clone(), config.jwt.clone()));
    let email_service = Arc::new(EmailService::new(config.email.clone()));
    let admin_service = Arc::new(AdminService::new(db_pool.clone(), config.jwt.secret.clone()));
    let proxy_config_service = Arc::new(ProxyConfigService::new(db_pool.clone()));

    let config_cache = Arc::new(ProxyConfigCache::new(
        format!("127.0.0.1:{}", config.server.api_port),
    ));
    load_proxy_config(&proxy_config_service, &config_cache).await?;

    initialize_admin_token(&admin_service).await?;

    let request_counter = Arc::new(AtomicU64::new(0));

    let state = AppState {
        db_pool: db_pool.clone(),
        user_service,
        token_service,
        email_service,
        admin_service,
        proxy_config_service,
        request_counter,
    };

    let api_port = config.server.api_port;
    tokio::spawn(async move {
        let app = api::create_router(state);
        let listener = TcpListener::bind(format!("0.0.0.0:{}", api_port))
            .await
            .expect("Failed to bind API port");
        tracing::info!("Auth API listening on 0.0.0.0:{}", api_port);
        axum::serve(listener, app).await.expect("API server failed");
    });

    tracing::info!("Starting Pingora gateway on 0.0.0.0:{}", config.server.gateway_port);
    
    std::thread::spawn(move || {
        start_gateway(config, jwt_validator, config_cache);
    });

    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");

    Ok(())
}

fn init_logging() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn initialize_admin_token(admin_service: &AdminService) -> anyhow::Result<()> {
    let admin_count = admin_service.count().await?;
    let has_valid_token = admin_service.has_valid_registration_token().await?;

    if admin_count == 0 && !has_valid_token {
        let token = admin_service.generate_registration_token().await?;
        tracing::info!("========================================");
        tracing::info!("NO ADMIN FOUND - REGISTRATION TOKEN GENERATED");
        tracing::info!("Token: {}", token);
        tracing::info!("Valid for 24 hours. Use this to register the first admin.");
        tracing::info!("========================================");
    }

    Ok(())
}

fn start_gateway(config: Arc<AppConfig>, jwt_validator: Arc<JwtValidator>, config_cache: Arc<ProxyConfigCache>) {
    use pingora::server::Server;
    use pingora::proxy::http_proxy_service;
    use gateway::proxy::AuthGateway;

    let mut server = Server::new(None).expect("Failed to create server");
    server.bootstrap();

    let gateway = AuthGateway::new(jwt_validator, config_cache);

    let mut proxy = http_proxy_service(&server.configuration, gateway);
    proxy.add_tcp(&format!("0.0.0.0:{}", config.server.gateway_port));

    server.add_service(proxy);
    server.run_forever();
}

async fn load_proxy_config(
    service: &ProxyConfigService,
    cache: &ProxyConfigCache,
) -> anyhow::Result<()> {
    let routes = service.list_routes().await?;

    let cached_routes: Vec<CachedRoute> = routes
        .into_iter()
        .filter(|r| r.enabled)
        .map(|r| CachedRoute {
            path_prefix: r.path_prefix,
            upstream_address: r.upstream_address,
            require_auth: r.require_auth,
        })
        .collect();

    cache.update_routes(cached_routes);
    tracing::info!("Loaded proxy configuration from database");
    Ok(())
}
