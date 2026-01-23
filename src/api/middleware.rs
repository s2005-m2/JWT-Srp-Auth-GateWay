use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<RateLimiterInner>,
}

struct RateLimiterInner {
    windows: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            inner: Arc::new(RateLimiterInner {
                windows: Mutex::new(HashMap::new()),
                max_requests,
                window_duration: Duration::from_secs(window_secs),
            }),
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut windows = self.inner.windows.lock().unwrap();

        let timestamps = windows.entry(key.to_string()).or_default();
        timestamps.retain(|t| now.duration_since(*t) < self.inner.window_duration);

        if timestamps.len() >= self.inner.max_requests {
            return false;
        }

        timestamps.push(now);
        true
    }
}

#[derive(Serialize)]
struct RateLimitError {
    error: RateLimitErrorBody,
}

#[derive(Serialize)]
struct RateLimitErrorBody {
    code: &'static str,
    message: &'static str,
}

pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
    rate_limiter: RateLimiter,
) -> Response {
    let key = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if !rate_limiter.check(&key) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: RateLimitErrorBody {
                    code: "RATE_LIMITED",
                    message: "Rate limit exceeded",
                },
            }),
        )
            .into_response();
    }

    next.run(request).await
}

pub async fn request_counter_middleware(
    request: Request,
    next: Next,
    counter: Arc<AtomicU64>,
) -> Response {
    counter.fetch_add(1, Ordering::Relaxed);
    next.run(request).await
}
