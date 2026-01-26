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
use services::{AdminService, EmailService, ProxyConfigService, SystemConfigService, TokenService, UserService};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();

    let config = AppConfig::load()?;
    let db_pool = db::create_pool(&config.database).await?;

    tracing::info!("Running database migrations...");
    db::run_migrations(&db_pool).await?;

    let system_config_service = Arc::new(SystemConfigService::new(db_pool.clone()));
    system_config_service.initialize().await?;

    let jwt_validator = Arc::new(JwtValidator::new(
        system_config_service.clone(),
        config.jwt.auto_refresh_threshold,
    ));
    jwt_validator.init().await?;
    let user_service = Arc::new(UserService::new(db_pool.clone()));
    let token_service = Arc::new(TokenService::new(
        db_pool.clone(),
        system_config_service.clone(),
        config.jwt.access_token_ttl,
        config.jwt.refresh_token_ttl,
        config.jwt.auto_refresh_threshold,
    ));
    let email_service = Arc::new(EmailService::new(system_config_service.clone()));
    let admin_service = Arc::new(AdminService::new(db_pool.clone(), system_config_service.clone()));
    let proxy_config_service = Arc::new(ProxyConfigService::new(db_pool.clone()));

    let default_upstream = if config.upstream.default_upstream.is_empty() {
        None
    } else {
        Some(config.upstream.default_upstream.clone())
    };
    let mut config_cache = ProxyConfigCache::new(
        format!("127.0.0.1:{}", config.server.api_port),
        default_upstream,
    );

    let static_routes: Vec<CachedRoute> = config
        .routing
        .routes
        .iter()
        .map(|r| CachedRoute {
            path_prefix: r.path.clone(),
            upstream_address: r.upstream.clone(),
            require_auth: r.auth,
            strip_prefix: r.strip_prefix.clone(),
        })
        .collect();
    if !static_routes.is_empty() {
        tracing::info!("Loaded {} static routes from config/env", static_routes.len());
        config_cache.set_static_routes(static_routes);
    }

    let config_cache = Arc::new(config_cache);
    load_proxy_config(&proxy_config_service, &config_cache).await?;

    initialize_admin_token(&admin_service).await?;

    let request_counter = Arc::new(AtomicU64::new(0));

    let system_config_for_scheduler = system_config_service.clone();
    let jwt_validator_for_scheduler = jwt_validator.clone();

    let state = AppState {
        db_pool: db_pool.clone(),
        user_service,
        token_service,
        email_service,
        admin_service,
        proxy_config_service,
        system_config_service,
        jwt_validator: Some(jwt_validator.clone()),
        config_cache: Some(config_cache.clone()),
        request_counter,
    };

    tokio::spawn(async move {
        jwt_rotation_scheduler(system_config_for_scheduler, jwt_validator_for_scheduler).await;
    });

    let db_pool_for_cleanup = db_pool.clone();
    tokio::spawn(async move {
        database_cleanup_scheduler(db_pool_for_cleanup).await;
    });

    let api_port = config.server.api_port;
    tokio::spawn(async move {
        if let Err(e) = run_api_server(state, api_port).await {
            tracing::error!("API server failed: {}", e);
        }
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

async fn run_api_server(state: AppState, port: u16) -> anyhow::Result<()> {
    let app = api::create_router(state);
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    tracing::info!("Auth API listening on 127.0.0.1:{} (internal only)", port);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn initialize_admin_token(admin_service: &AdminService) -> anyhow::Result<()> {
    let admin_count = admin_service.count().await?;

    if admin_count == 0 {
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

    let mut server = match Server::new(None) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to create Pingora server: {}", e);
            return;
        }
    };
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
            strip_prefix: r.strip_prefix,
        })
        .collect();

    let routes_count = cached_routes.len();
    cache.update_routes(cached_routes);
    tracing::info!("Loaded {} dynamic routes from database", routes_count);
    Ok(())
}

async fn database_cleanup_scheduler(db_pool: Arc<sqlx::PgPool>) {
    use tokio::time::{interval, Duration};
    
    let mut cleanup_interval = interval(Duration::from_secs(60 * 60));
    
    loop {
        cleanup_interval.tick().await;
        
        let deleted_codes = sqlx::query("DELETE FROM verification_codes WHERE expires_at < NOW()")
            .execute(db_pool.as_ref())
            .await;
        
        match deleted_codes {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!("Cleaned up {} expired verification codes", result.rows_affected());
                }
            }
            Err(e) => tracing::error!("Failed to cleanup verification codes: {}", e),
        }
        
        let deleted_tokens = sqlx::query(
            "DELETE FROM refresh_tokens WHERE expires_at < NOW() OR revoked = TRUE"
        )
            .execute(db_pool.as_ref())
            .await;
        
        match deleted_tokens {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!("Cleaned up {} expired/revoked refresh tokens", result.rows_affected());
                }
            }
            Err(e) => tracing::error!("Failed to cleanup refresh tokens: {}", e),
        }
    }
}

async fn jwt_rotation_scheduler(
    system_config: Arc<SystemConfigService>,
    jwt_validator: Arc<JwtValidator>,
) {
    use tokio::time::{interval, Duration};
    
    let mut check_interval = interval(Duration::from_secs(24 * 60 * 60));
    
    loop {
        check_interval.tick().await;
        
        match system_config.should_auto_rotate().await {
            Ok(true) => {
                tracing::info!("JWT secret is older than 30 days, rotating...");
                if let Err(e) = system_config.rotate_jwt_secret().await {
                    tracing::error!("Failed to auto-rotate JWT secret: {}", e);
                    continue;
                }
                if let Err(e) = jwt_validator.refresh_secret().await {
                    tracing::error!("Failed to refresh JWT validator: {}", e);
                }
                tracing::info!("JWT secret auto-rotated successfully");
            }
            Ok(false) => {
                tracing::debug!("JWT secret is still fresh, no rotation needed");
            }
            Err(e) => {
                tracing::error!("Failed to check JWT rotation status: {}", e);
            }
        }
    }
}
