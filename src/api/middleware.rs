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
    last_cleanup: Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            inner: Arc::new(RateLimiterInner {
                windows: Mutex::new(HashMap::new()),
                max_requests,
                window_duration: Duration::from_secs(window_secs),
                last_cleanup: Mutex::new(Instant::now()),
            }),
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        self.maybe_cleanup(now);
        
        let mut windows = match self.inner.windows.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let timestamps = windows.entry(key.to_string()).or_default();
        timestamps.retain(|t| now.duration_since(*t) < self.inner.window_duration);

        if timestamps.len() >= self.inner.max_requests {
            return false;
        }

        timestamps.push(now);
        true
    }
    
    fn maybe_cleanup(&self, now: Instant) {
        let cleanup_interval = Duration::from_secs(60);
        
        let should_cleanup = {
            let last = match self.inner.last_cleanup.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            now.duration_since(*last) > cleanup_interval
        };
        
        if !should_cleanup {
            return;
        }
        
        let mut last = match self.inner.last_cleanup.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        
        if now.duration_since(*last) <= cleanup_interval {
            return;
        }
        *last = now;
        drop(last);
        
        let mut windows = match self.inner.windows.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        
        windows.retain(|_, timestamps| {
            timestamps.retain(|t| now.duration_since(*t) < self.inner.window_duration);
            !timestamps.is_empty()
        });
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
    let key = extract_client_ip(&request);

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

fn extract_client_ip(request: &Request) -> String {
    request
        .headers()
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

pub async fn request_counter_middleware(
    request: Request,
    next: Next,
    counter: Arc<AtomicU64>,
) -> Response {
    counter.fetch_add(1, Ordering::Relaxed);
    next.run(request).await
}
