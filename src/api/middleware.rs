use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use axum::{extract::Request, middleware::Next, response::Response};

pub struct RateLimiter {
    windows: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
            max_requests,
            window_duration: Duration::from_secs(window_secs),
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut windows = self.windows.lock().unwrap();

        let timestamps = windows.entry(key.to_string()).or_insert_with(Vec::new);
        timestamps.retain(|t| now.duration_since(*t) < self.window_duration);

        if timestamps.len() >= self.max_requests {
            return false;
        }

        timestamps.push(now);
        true
    }
}

pub async fn request_counter_middleware(
    request: Request,
    next: Next,
    counter: Arc<AtomicU64>,
) -> Response {
    counter.fetch_add(1, Ordering::Relaxed);
    next.run(request).await
}
