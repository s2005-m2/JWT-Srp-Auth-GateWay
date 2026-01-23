use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CachedRoute {
    pub path_prefix: String,
    pub upstream_address: String,
    pub strip_prefix: bool,
    pub require_auth: bool,
    pub priority: i32,
}

#[derive(Debug, Clone)]
pub struct CachedUpstream {
    pub id: Uuid,
    pub name: String,
    pub address: String,
}

pub struct ProxyConfigCache {
    routes: RwLock<Vec<CachedRoute>>,
    upstreams: RwLock<HashMap<Uuid, CachedUpstream>>,
    auth_upstream: String,
}

impl ProxyConfigCache {
    pub fn new(auth_upstream: String) -> Self {
        Self {
            routes: RwLock::new(Vec::new()),
            upstreams: RwLock::new(HashMap::new()),
            auth_upstream,
        }
    }

    pub fn update_config(&self, routes: Vec<CachedRoute>, upstreams: Vec<CachedUpstream>) {
        {
            let mut cached_routes = self.routes.write().unwrap();
            *cached_routes = routes;
        }
        {
            let mut cached_upstreams = self.upstreams.write().unwrap();
            cached_upstreams.clear();
            for upstream in upstreams {
                cached_upstreams.insert(upstream.id, upstream);
            }
        }
    }

    pub fn match_route(&self, path: &str) -> Option<MatchedRoute> {
        if path.starts_with("/auth/")
            || path.starts_with("/api/admin")
            || path.starts_with("/api/config")
        {
            return Some(MatchedRoute {
                upstream_address: self.auth_upstream.clone(),
                strip_prefix: false,
                require_auth: false,
            });
        }

        let routes = self.routes.read().unwrap();
        for route in routes.iter() {
            if path.starts_with(&route.path_prefix) {
                return Some(MatchedRoute {
                    upstream_address: route.upstream_address.clone(),
                    strip_prefix: route.strip_prefix,
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
    pub strip_prefix: bool,
    pub require_auth: bool,
}
