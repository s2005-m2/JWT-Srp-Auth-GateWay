use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct CachedRoute {
    pub path_prefix: String,
    pub upstream_address: String,
    pub require_auth: bool,
}

pub struct ProxyConfigCache {
    routes: RwLock<Vec<CachedRoute>>,
    auth_upstream: String,
    default_upstream: Option<String>,
}

impl ProxyConfigCache {
    pub fn new(auth_upstream: String, default_upstream: Option<String>) -> Self {
        Self {
            routes: RwLock::new(Vec::new()),
            auth_upstream,
            default_upstream,
        }
    }

    pub fn update_routes(&self, routes: Vec<CachedRoute>) {
        let mut cached_routes = self.routes.write().unwrap();
        *cached_routes = routes;
    }

    pub fn match_route(&self, path: &str) -> Option<MatchedRoute> {
        if path.starts_with("/arc-admin/") || path == "/arc-admin" {
            let require_auth = !path.starts_with("/arc-admin/auth/");
            return Some(MatchedRoute {
                upstream_address: self.auth_upstream.clone(),
                require_auth,
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

        let routes = self.routes.read().unwrap();
        for route in routes.iter() {
            if path.starts_with(&route.path_prefix) {
                return Some(MatchedRoute {
                    upstream_address: route.upstream_address.clone(),
                    require_auth: route.require_auth,
                    strip_prefix: None,
                });
            }
        }

        self.default_upstream.as_ref().map(|upstream| MatchedRoute {
            upstream_address: upstream.clone(),
            require_auth: false,
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
