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
}

impl ProxyConfigCache {
    pub fn new(auth_upstream: String, default_upstream: Option<String>) -> Self {
        Self {
            static_routes: Vec::new(),
            dynamic_routes: RwLock::new(Vec::new()),
            auth_upstream,
            default_upstream,
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
}

#[derive(Debug, Clone)]
pub struct MatchedRoute {
    pub upstream_address: String,
    pub require_auth: bool,
    pub strip_prefix: Option<String>,
}
