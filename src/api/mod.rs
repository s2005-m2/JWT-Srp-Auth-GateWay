use axum::{middleware as axum_middleware, routing::{get, post, put}, Router};
use sqlx::PgPool;
use std::sync::{atomic::AtomicU64, Arc};
use tower_http::services::{ServeDir, ServeFile};

use middleware::{rate_limit_middleware, request_counter_middleware, RateLimiter};

use crate::gateway::{JwtValidator, ProxyConfigCache};
use crate::services::{AdminService, EmailService, ProxyConfigService, SystemConfigService, TokenService, UserService};

pub mod handlers;
pub mod middleware;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<PgPool>,
    pub user_service: Arc<UserService>,
    pub token_service: Arc<TokenService>,
    pub email_service: Arc<EmailService>,
    pub admin_service: Arc<AdminService>,
    pub proxy_config_service: Arc<ProxyConfigService>,
    pub system_config_service: Arc<SystemConfigService>,
    pub jwt_validator: Option<Arc<JwtValidator>>,
    pub config_cache: Option<Arc<ProxyConfigCache>>,
    pub request_counter: Arc<AtomicU64>,
}

pub fn create_router(state: AppState) -> Router {
    let serve_dir = ServeDir::new("web/dist")
        .not_found_service(ServeFile::new("web/dist/index.html"));

    let counter = state.request_counter.clone();
    let global_rate_limiter = RateLimiter::new(100, 60);

    let protected_admin_routes = Router::new()
        .route("/stats", get(handlers::get_stats))
        .route("/users", get(handlers::get_users))
        .route("/activities", get(handlers::get_activities))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            handlers::admin_auth_middleware,
        ));

    let admin_routes = Router::new()
        .route("/login", post(handlers::admin_login))
        .route("/register", post(handlers::admin_register))
        .merge(protected_admin_routes);

    let config_routes = Router::new()
        .route("/routes", get(handlers::list_routes).post(handlers::create_route))
        .route("/routes/:id", put(handlers::update_route).delete(handlers::delete_route))
        .route("/rate-limits", get(handlers::list_rate_limits).post(handlers::create_rate_limit))
        .route("/rate-limits/:id", put(handlers::update_rate_limit).delete(handlers::delete_rate_limit))
        .route("/jwt", get(handlers::get_jwt_config).put(handlers::update_jwt_config))
        .route("/smtp", get(handlers::get_smtp_config).put(handlers::update_smtp_config))
        .route("/jwt-secret", get(handlers::get_jwt_secret_info).post(handlers::rotate_jwt_secret))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            handlers::admin_auth_middleware,
        ));

    let auth_limiter = RateLimiter::new(10, 60);
    let auth_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/register/verify", post(handlers::verify))
        .route("/login", post(handlers::login))
        .route("/refresh", post(handlers::refresh))
        .route("/password/reset", post(handlers::request_password_reset))
        .route("/password/reset/confirm", post(handlers::reset_password))
        .layer(axum_middleware::from_fn(move |req, next| {
            rate_limit_middleware(req, next, auth_limiter.clone())
        }));

    Router::new()
        .nest("/auth", auth_routes)
        .nest("/api/admin", admin_routes)
        .nest("/api/config", config_routes)
        .fallback_service(serve_dir)
        .with_state(state)
        .layer(axum_middleware::from_fn(move |req, next| {
            rate_limit_middleware(req, next, global_rate_limiter.clone())
        }))
        .layer(axum_middleware::from_fn(move |req, next| {
            request_counter_middleware(req, next, counter.clone())
        }))
}
