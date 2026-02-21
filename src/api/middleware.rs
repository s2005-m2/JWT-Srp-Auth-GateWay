use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, Request},
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
    let key = match extract_client_ip(&request) {
        Some(ip) => ip,
        None => {
            tracing::warn!(
                "Request rejected: unable to determine client IP (ConnectInfo not configured)"
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(RateLimitError {
                    error: RateLimitErrorBody {
                        code: "INVALID_REQUEST",
                        message: "Unable to determine client IP",
                    },
                }),
            )
                .into_response();
        }
    };

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

const TRUSTED_PROXIES: &[&str] = &[
    "127.0.0.1",
    "::1",
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
];

fn extract_client_ip(request: &Request) -> Option<String> {
    let connect_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip());

    let connect_ip = connect_ip?;

    let is_trusted = is_trusted_proxy(&connect_ip);

    if is_trusted {
        if let Some(real_ip) = request
            .headers()
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
        {
            return Some(real_ip.trim().to_string());
        }

        if let Some(forwarded) = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
        {
            if let Some(first_ip) = forwarded.split(',').next() {
                let ip = first_ip.trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
    }

    Some(connect_ip.to_string())
}

fn is_trusted_proxy(ip: &IpAddr) -> bool {
    let ip_str = ip.to_string();

    for trusted in TRUSTED_PROXIES {
        if trusted.contains('/') {
            if let Some((network, prefix_len)) = trusted.split_once('/') {
                if let (Ok(net_ip), Ok(prefix)) =
                    (network.parse::<IpAddr>(), prefix_len.parse::<u8>())
                {
                    if ip_in_cidr(ip, &net_ip, prefix) {
                        return true;
                    }
                }
            }
        } else if ip_str == *trusted {
            return true;
        }
    }
    false
}

fn ip_in_cidr(ip: &IpAddr, network: &IpAddr, prefix_len: u8) -> bool {
    match (ip, network) {
        (IpAddr::V4(ip), IpAddr::V4(net)) => {
            let ip_bits = u32::from(*ip);
            let net_bits = u32::from(*net);
            let mask = if prefix_len >= 32 {
                u32::MAX
            } else {
                u32::MAX << (32 - prefix_len)
            };
            (ip_bits & mask) == (net_bits & mask)
        }
        (IpAddr::V6(ip), IpAddr::V6(net)) => {
            let ip_bits = u128::from(*ip);
            let net_bits = u128::from(*net);
            let mask = if prefix_len >= 128 {
                u128::MAX
            } else {
                u128::MAX << (128 - prefix_len)
            };
            (ip_bits & mask) == (net_bits & mask)
        }
        _ => false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn test_is_trusted_proxy_localhost() {
        let localhost_v4: IpAddr = "127.0.0.1".parse().unwrap();
        let localhost_v6: IpAddr = "::1".parse().unwrap();

        assert!(is_trusted_proxy(&localhost_v4));
        assert!(is_trusted_proxy(&localhost_v6));
    }

    #[test]
    fn test_is_trusted_proxy_private_networks() {
        let ip_10: IpAddr = "10.0.0.1".parse().unwrap();
        let ip_172: IpAddr = "172.16.0.1".parse().unwrap();
        let ip_192: IpAddr = "192.168.1.1".parse().unwrap();

        assert!(is_trusted_proxy(&ip_10));
        assert!(is_trusted_proxy(&ip_172));
        assert!(is_trusted_proxy(&ip_192));
    }

    #[test]
    fn test_is_trusted_proxy_public_ip_rejected() {
        let public_ip: IpAddr = "8.8.8.8".parse().unwrap();
        let another_public: IpAddr = "203.0.113.1".parse().unwrap();

        assert!(!is_trusted_proxy(&public_ip));
        assert!(!is_trusted_proxy(&another_public));
    }

    #[test]
    fn test_ip_in_cidr_ipv4() {
        let network: IpAddr = "10.0.0.0".parse().unwrap();

        let in_range: IpAddr = "10.255.255.255".parse().unwrap();
        let out_of_range: IpAddr = "11.0.0.0".parse().unwrap();

        assert!(ip_in_cidr(&in_range, &network, 8));
        assert!(!ip_in_cidr(&out_of_range, &network, 8));
    }

    #[test]
    fn test_ip_in_cidr_ipv4_24() {
        let network: IpAddr = "192.168.1.0".parse().unwrap();

        let in_range: IpAddr = "192.168.1.254".parse().unwrap();
        let out_of_range: IpAddr = "192.168.2.1".parse().unwrap();

        assert!(ip_in_cidr(&in_range, &network, 24));
        assert!(!ip_in_cidr(&out_of_range, &network, 24));
    }

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(5, 60);

        for _ in 0..5 {
            assert!(limiter.check("test_key"));
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(3, 60);

        assert!(limiter.check("test_key"));
        assert!(limiter.check("test_key"));
        assert!(limiter.check("test_key"));
        assert!(!limiter.check("test_key"));
    }

    #[test]
    fn test_rate_limiter_separate_keys() {
        let limiter = RateLimiter::new(2, 60);

        assert!(limiter.check("key1"));
        assert!(limiter.check("key1"));
        assert!(!limiter.check("key1"));

        assert!(limiter.check("key2"));
        assert!(limiter.check("key2"));
    }

    #[test]
    fn test_extract_client_ip_returns_none_without_connect_info() {
        let request = Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_client_ip(&request);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_client_ip_returns_ip_with_connect_info() {
        let mut request = Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let addr: SocketAddr = "8.8.8.8:12345".parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(addr));

        let result = extract_client_ip(&request);
        assert_eq!(result, Some("8.8.8.8".to_string()));
    }

    #[test]
    fn test_extract_client_ip_trusts_x_real_ip_from_trusted_proxy() {
        let mut request = Request::builder()
            .uri("/test")
            .header("x-real-ip", "203.0.113.50")
            .body(axum::body::Body::empty())
            .unwrap();

        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(addr));

        let result = extract_client_ip(&request);
        assert_eq!(result, Some("203.0.113.50".to_string()));
    }

    #[test]
    fn test_extract_client_ip_ignores_x_real_ip_from_untrusted_source() {
        let mut request = Request::builder()
            .uri("/test")
            .header("x-real-ip", "203.0.113.50")
            .body(axum::body::Body::empty())
            .unwrap();

        let addr: SocketAddr = "8.8.8.8:12345".parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(addr));

        let result = extract_client_ip(&request);
        assert_eq!(result, Some("8.8.8.8".to_string()));
    }
}
