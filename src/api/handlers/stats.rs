use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::sync::atomic::Ordering;
use uuid::Uuid;

use crate::api::AppState;
use crate::error::{AppError, Result};

#[derive(Serialize)]
pub struct StatsResponse {
    pub active_users: i64,
    pub total_requests: u64,
    pub system_status: String,
    pub server_start_time: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct UserListItem {
    pub id: String,
    pub email: String,
    pub email_verified: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
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
        "SELECT COUNT(*)::BIGINT FROM users WHERE created_at > NOW() - INTERVAL '30 days'"
    )
    .fetch_one(state.db_pool.as_ref())
    .await?;

    let total_requests = state.request_counter.load(Ordering::Relaxed);

    Ok(Json(StatsResponse {
        active_users: active_users.0,
        total_requests,
        system_status: "healthy".to_string(),
        server_start_time: get_server_start_time(),
    }))
}

pub async fn get_users(
    State(state): State<AppState>,
) -> Result<Json<UserListResponse>> {
    let users: Vec<(uuid::Uuid, String, bool, bool, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, email, email_verified, is_active, created_at FROM users ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(state.db_pool.as_ref())
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*)::BIGINT FROM users")
        .fetch_one(state.db_pool.as_ref())
        .await?;

    let user_list: Vec<UserListItem> = users
        .into_iter()
        .map(|(id, email, email_verified, is_active, created_at)| UserListItem {
            id: id.to_string(),
            email,
            email_verified,
            is_active,
            created_at,
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

#[derive(Deserialize)]
pub struct UpdateUserStatusRequest {
    pub is_active: bool,
}

pub async fn update_user_status(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserStatusRequest>,
) -> Result<Json<serde_json::Value>> {
    let result = sqlx::query("UPDATE users SET is_active = $1, updated_at = NOW() WHERE id = $2")
        .bind(req.is_active)
        .bind(user_id)
        .execute(state.db_pool.as_ref())
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(state.db_pool.as_ref())
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
