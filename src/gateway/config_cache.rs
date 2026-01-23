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
}

impl ProxyConfigCache {
    pub fn new(auth_upstream: String) -> Self {
        Self {
            routes: RwLock::new(Vec::new()),
            auth_upstream,
        }
    }

    pub fn update_routes(&self, routes: Vec<CachedRoute>) {
        let mut cached_routes = self.routes.write().unwrap();
        *cached_routes = routes;
    }

    pub fn match_route(&self, path: &str) -> Option<MatchedRoute> {
        if path.starts_with("/auth/") {
            return Some(MatchedRoute {
                upstream_address: self.auth_upstream.clone(),
                require_auth: false,
            });
        }

        if path.starts_with("/api/admin") || path.starts_with("/api/config") {
            return Some(MatchedRoute {
                upstream_address: self.auth_upstream.clone(),
                require_auth: true,
            });
        }

        let routes = self.routes.read().unwrap();
        for route in routes.iter() {
            if path.starts_with(&route.path_prefix) {
                return Some(MatchedRoute {
                    upstream_address: route.upstream_address.clone(),
                    require_auth: route.require_auth,
                });
            }
        }
        None
    }

    pub fn auth_upstream(&self) -> &str {
        &self.auth_upstream
    }
}

#[derive(Debug, Clone)]
pub struct MatchedRoute {
    pub upstream_address: String,
    pub require_auth: bool,
}
