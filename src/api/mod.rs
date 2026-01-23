use axum::{middleware as axum_middleware, routing::{get, post, put}, Router};
use sqlx::PgPool;
use std::sync::{atomic::AtomicU64, Arc};
use tower_http::services::{ServeDir, ServeFile};

use middleware::request_counter_middleware;

use crate::services::{AdminService, EmailService, ProxyConfigService, TokenService, UserService};

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
    pub request_counter: Arc<AtomicU64>,
}

pub fn create_router(state: AppState) -> Router {
    let serve_dir = ServeDir::new("web/dist")
        .not_found_service(ServeFile::new("web/dist/index.html"));

    let counter = state.request_counter.clone();

    let admin_routes = Router::new()
        .route("/login", post(handlers::admin_login))
        .route("/register", post(handlers::admin_register))
        .route("/stats", get(handlers::get_stats))
        .route("/users", get(handlers::get_users))
        .route("/activities", get(handlers::get_activities));

    let config_routes = Router::new()
        .route("/routes", get(handlers::list_routes).post(handlers::create_route))
        .route("/routes/{id}", put(handlers::update_route).delete(handlers::delete_route))
        .route("/rate-limits", get(handlers::list_rate_limits).post(handlers::create_rate_limit))
        .route("/rate-limits/{id}", put(handlers::update_rate_limit).delete(handlers::delete_rate_limit))
        .route("/jwt", get(handlers::get_jwt_config).put(handlers::update_jwt_config));

    Router::new()
        .route("/auth/register", post(handlers::register))
        .route("/auth/register/verify", post(handlers::verify))
        .route("/auth/login", post(handlers::login))
        .route("/auth/refresh", post(handlers::refresh))
        .nest("/api/admin", admin_routes)
        .nest("/api/config", config_routes)
        .fallback_service(serve_dir)
        .with_state(state)
        .layer(axum_middleware::from_fn(move |req, next| {
            request_counter_middleware(req, next, counter.clone())
        }))
}
