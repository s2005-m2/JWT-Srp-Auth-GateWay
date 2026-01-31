use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct CachedRoute {
    pub path_prefix: String,
    pub upstream_address: String,
    pub require_auth: bool,
    pub strip_prefix: Option<String>,
}

pub struct ProxyConfigCache {
    static_routes: Vec<CachedRoute>,
    dynamic_routes: RwLock<Vec<CachedRoute>>,
    auth_upstream: String,
    default_upstream: Option<String>,
    /// Pre-resolved DNS cache: "host:port" -> SocketAddr
    resolved_addrs: RwLock<HashMap<String, SocketAddr>>,
}

impl ProxyConfigCache {
    pub fn new(auth_upstream: String, default_upstream: Option<String>) -> Self {
        Self {
            static_routes: Vec::new(),
            dynamic_routes: RwLock::new(Vec::new()),
            auth_upstream,
            default_upstream,
            resolved_addrs: RwLock::new(HashMap::new()),
        }
    }

    pub fn set_static_routes(&mut self, routes: Vec<CachedRoute>) {
        self.static_routes = routes;
    }

    pub fn update_routes(&self, routes: Vec<CachedRoute>) {
        if let Ok(mut dynamic) = self.dynamic_routes.write() {
            *dynamic = routes;
        }
    }

    pub fn match_route(&self, path: &str) -> Option<MatchedRoute> {
        if path.starts_with("/.well-known/") {
            return None;
        }

        if path.starts_with("/arc-admin/") || path == "/arc-admin" {
            return Some(MatchedRoute {
                upstream_address: self.auth_upstream.clone(),
                require_auth: false,
                strip_prefix: Some("/arc-admin".to_string()),
            });
        }

        if path.starts_with("/auth/") {
            return Some(MatchedRoute {
                upstream_address: self.auth_upstream.clone(),
                require_auth: false,
                strip_prefix: None,
            });
        }

        if path.starts_with("/api/admin") || path.starts_with("/api/config") {
            return Some(MatchedRoute {
                upstream_address: self.auth_upstream.clone(),
                require_auth: true,
                strip_prefix: None,
            });
        }

        for route in &self.static_routes {
            if path.starts_with(&route.path_prefix) {
                return Some(MatchedRoute {
                    upstream_address: route.upstream_address.clone(),
                    require_auth: route.require_auth,
                    strip_prefix: route.strip_prefix.clone(),
                });
            }
        }

        let dynamic = match self.dynamic_routes.read() {
            Ok(guard) => guard,
            Err(e) => {
                tracing::warn!("Failed to acquire dynamic routes lock: {}", e);
                return self.default_upstream.as_ref().map(|upstream| MatchedRoute {
                    upstream_address: upstream.clone(),
                    require_auth: true,
                    strip_prefix: None,
                });
            }
        };
        for route in dynamic.iter() {
            if path.starts_with(&route.path_prefix) {
                return Some(MatchedRoute {
                    upstream_address: route.upstream_address.clone(),
                    require_auth: route.require_auth,
                    strip_prefix: route.strip_prefix.clone(),
                });
            }
        }

        self.default_upstream.as_ref().map(|upstream| MatchedRoute {
            upstream_address: upstream.clone(),
            require_auth: true,
            strip_prefix: None,
        })
    }

    pub fn auth_upstream(&self) -> &str {
        &self.auth_upstream
    }

    pub fn resolve_all_upstreams(&self) {
        let mut addrs_to_resolve = vec![self.auth_upstream.clone()];

        if let Some(ref default) = self.default_upstream {
            addrs_to_resolve.push(default.clone());
        }

        for route in &self.static_routes {
            addrs_to_resolve.push(route.upstream_address.clone());
        }

        if let Ok(dynamic) = self.dynamic_routes.read() {
            for route in dynamic.iter() {
                addrs_to_resolve.push(route.upstream_address.clone());
            }
        }

        let mut resolved = HashMap::new();
        for addr in addrs_to_resolve {
            if let Some(socket_addr) = Self::resolve_address(&addr) {
                tracing::info!(upstream = %addr, resolved = %socket_addr, "DNS pre-resolved");
                resolved.insert(addr, socket_addr);
            }
        }

        if let Ok(mut cache) = self.resolved_addrs.write() {
            *cache = resolved;
        }
    }

    fn resolve_address(addr: &str) -> Option<SocketAddr> {
        match addr.to_socket_addrs() {
            Ok(mut addrs) => addrs.next(),
            Err(e) => {
                tracing::warn!(upstream = %addr, error = %e, "DNS resolution failed");
                None
            }
        }
    }

    pub fn get_resolved_addr(&self, addr: &str) -> Option<SocketAddr> {
        self.resolved_addrs
            .read()
            .ok()
            .and_then(|cache| cache.get(addr).copied())
    }
}

#[derive(Debug, Clone)]
pub struct MatchedRoute {
    pub upstream_address: String,
    pub require_auth: bool,
    pub strip_prefix: Option<String>,
}
