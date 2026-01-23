use axum::{extract::State, Json};
use serde::Serialize;
use chrono::{DateTime, Utc};

use crate::api::AppState;
use crate::error::Result;

#[derive(Serialize)]
pub struct StatsResponse {
    pub active_users: i64,
    pub total_requests: i64,
    pub system_status: String,
    pub uptime_percent: f64,
    pub server_start_time: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct UserListItem {
    pub id: String,
    pub email: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct UserListResponse {
    pub users: Vec<UserListItem>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct ActivityItem {
    pub id: String,
    pub action: String,
    pub email: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ActivitiesResponse {
    pub activities: Vec<ActivityItem>,
}

static SERVER_START_TIME: std::sync::OnceLock<DateTime<Utc>> = std::sync::OnceLock::new();

fn get_server_start_time() -> DateTime<Utc> {
    *SERVER_START_TIME.get_or_init(Utc::now)
}

pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<StatsResponse>> {
    let active_users: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM users WHERE created_at > NOW() - INTERVAL '30 days'"
    )
    .fetch_one(state.db_pool.as_ref())
    .await?;

    Ok(Json(StatsResponse {
        active_users: active_users.0,
        total_requests: 0,
        system_status: "healthy".to_string(),
        uptime_percent: 99.9,
        server_start_time: get_server_start_time(),
    }))
}

pub async fn get_users(
    State(state): State<AppState>,
) -> Result<Json<UserListResponse>> {
    let users: Vec<(uuid::Uuid, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, email, created_at FROM users ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(state.db_pool.as_ref())
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(state.db_pool.as_ref())
        .await?;

    let user_list: Vec<UserListItem> = users
        .into_iter()
        .map(|(id, email, created_at)| UserListItem {
            id: id.to_string(),
            email,
            status: "Active".to_string(),
            created_at,
            last_login: None,
        })
        .collect();

    Ok(Json(UserListResponse {
        users: user_list,
        total: total.0,
    }))
}

pub async fn get_activities(
    State(state): State<AppState>,
) -> Result<Json<ActivitiesResponse>> {
    let codes: Vec<(String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT email, code_type, created_at FROM verification_codes 
         ORDER BY created_at DESC LIMIT 10"
    )
    .fetch_all(state.db_pool.as_ref())
    .await?;

    let activities: Vec<ActivityItem> = codes
        .into_iter()
        .enumerate()
        .map(|(i, (email, code_type, created_at))| ActivityItem {
            id: i.to_string(),
            action: match code_type.as_str() {
                "register" => "User registration".to_string(),
                "reset" => "Password reset".to_string(),
                _ => code_type,
            },
            email,
            status: "Success".to_string(),
            created_at,
        })
        .collect();

    Ok(Json(ActivitiesResponse { activities }))
}
