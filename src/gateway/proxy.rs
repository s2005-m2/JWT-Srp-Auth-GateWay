use async_trait::async_trait;
use http::header::HeaderValue;
use pingora::http::{RequestHeader, ResponseHeader};
use pingora::prelude::HttpPeer;
use pingora::proxy::{ProxyHttp, Session};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::config_cache::{MatchedRoute, ProxyConfigCache};
use super::jwt::{JwtError, JwtValidator};

type Result<T> = pingora::Result<T>;

impl AuthGateway {
    async fn send_error(&self, session: &mut Session, status: u16, msg: &str) -> Result<bool> {
        let body = format!(r#"{{"error":{{"code":"{}","message":"{}"}}}}"#, status, msg);
        let mut header = ResponseHeader::build(status, None)?;
        header.insert_header("Content-Type", "application/json")?;
        header.insert_header("Content-Length", body.len().to_string())?;
        header.insert_header("Access-Control-Allow-Origin", "*")?;
        session.write_response_header(Box::new(header), true).await?;
        session.write_response_body(Some(body.into()), true).await?;
        Ok(true)
    }

    async fn send_cors_preflight(&self, session: &mut Session) -> Result<bool> {
        let mut header = ResponseHeader::build(204, None)?;
        header.insert_header("Access-Control-Allow-Origin", "*")?;
        header.insert_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")?;
        header.insert_header("Access-Control-Allow-Headers", "Content-Type, Authorization, X-API-Key")?;
        header.insert_header("Access-Control-Max-Age", "86400")?;
        session.write_response_header(Box::new(header), true).await?;
        Ok(true)
    }
}

pub struct AuthGateway {
    jwt_validator: Arc<JwtValidator>,
    config_cache: Arc<ProxyConfigCache>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionType {
    Http,
    WebSocket,
    Sse,
}

pub struct RequestCtx {
    pub user_id: Option<String>,
    pub request_id: String,
    pub should_refresh: bool,
    pub matched_route: Option<MatchedRoute>,
    pub connection_type: ConnectionType,
    pub origin: Option<String>,
}



impl AuthGateway {
    pub fn new(
        jwt_validator: Arc<JwtValidator>,
        config_cache: Arc<ProxyConfigCache>,
    ) -> Self {
        Self {
            jwt_validator,
            config_cache,
        }
    }

    fn extract_bearer_token(req: &RequestHeader) -> Option<&str> {
        req.headers
            .get("authorization")
            .and_then(|v: &HeaderValue| v.to_str().ok())
            .and_then(|s: &str| s.strip_prefix("Bearer "))
    }

    fn parse_upstream(addr: &str) -> (String, u16) {
        let parts: Vec<&str> = addr.split(':').collect();
        let host = parts[0].to_string();
        let port = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(80);
        (host, port)
    }

    fn detect_connection_type(req: &RequestHeader) -> ConnectionType {
        let dominated_upgrade = req
            .headers
            .get("upgrade")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_lowercase());

        if let Some(upgrade) = dominated_upgrade {
            if upgrade.contains("websocket") {
                return ConnectionType::WebSocket;
            }
        }

        let accept = req
            .headers
            .get("accept")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if accept.contains("text/event-stream") {
            return ConnectionType::Sse;
        }

        ConnectionType::Http
    }
}

#[async_trait]
impl ProxyHttp for AuthGateway {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        RequestCtx {
            user_id: None,
            request_id: Uuid::new_v4().to_string(),
            should_refresh: false,
            matched_route: None,
            connection_type: ConnectionType::Http,
            origin: None,
        }
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        let method = session.req_header().method.as_str();
        let path = session.req_header().uri.path();
        let query = session.req_header().uri.query().unwrap_or("");

        ctx.origin = session
            .req_header()
            .headers
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        if method == "OPTIONS" {
            return self.send_cors_preflight(session).await;
        }
        
        let headers = &session.req_header().headers;
        if headers.contains_key("x-user-id") || headers.contains_key("x-request-id") {
            warn!(
                req_id = %ctx.request_id,
                method = %method,
                path = %path,
                "Rejected: reserved header detected"
            );
            return self.send_error(session, 400, "Reserved header detected").await;
        }

        ctx.connection_type = Self::detect_connection_type(session.req_header());

        if ctx.connection_type != ConnectionType::Http {
            debug!(
                path = %path,
                conn_type = ?ctx.connection_type,
                "Long-lived connection detected, auth will be performed once"
            );
        }

        let matched = match self.config_cache.match_route(path) {
            Some(r) => r,
            None => {
                warn!(
                    req_id = %ctx.request_id,
                    method = %method,
                    path = %path,
                    "No route matched"
                );
                return self.send_error(session, 404, "Not found").await;
            }
        };

        info!(
            req_id = %ctx.request_id,
            method = %method,
            path = %path,
            query = %query,
            upstream = %matched.upstream_address,
            auth = %matched.require_auth,
            "Request received"
        );

        if matched.require_auth {
            let token = match Self::extract_bearer_token(session.req_header()) {
                Some(t) => t,
                None => {
                    warn!(
                        req_id = %ctx.request_id,
                        method = %method,
                        path = %path,
                        "Auth failed: missing token"
                    );
                    return self.send_error(session, 401, "Missing token").await;
                }
            };

            let claims = match self.jwt_validator.validate(token).await {
                Ok(c) => c,
                Err(JwtError::Expired) => {
                    warn!(
                        req_id = %ctx.request_id,
                        method = %method,
                        path = %path,
                        "Auth failed: token expired"
                    );
                    return self.send_error(session, 401, "Token expired").await;
                }
                Err(JwtError::Invalid) => {
                    warn!(
                        req_id = %ctx.request_id,
                        method = %method,
                        path = %path,
                        "Auth failed: invalid token"
                    );
                    return self.send_error(session, 401, "Invalid token").await;
                }
            };

            ctx.user_id = Some(claims.sub.to_string());
            ctx.should_refresh = self.jwt_validator.should_refresh(&claims);
        }

        ctx.matched_route = Some(matched);
        Ok(false)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let addr = ctx
            .matched_route
            .as_ref()
            .map(|r| r.upstream_address.as_str())
            .unwrap_or(self.config_cache.auth_upstream());

        let (host, port) = Self::parse_upstream(addr);
        let peer = HttpPeer::new((host.as_str(), port), false, String::new());
        Ok(Box::new(peer))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        if let Some(ref matched) = ctx.matched_route {
            if let Some(ref prefix) = matched.strip_prefix {
                let original_uri = upstream_request.uri.clone();
                let path = original_uri.path();
                let stripped = path.strip_prefix(prefix.as_str()).unwrap_or(path);
                
                let new_path = if stripped.is_empty() || !stripped.starts_with('/') {
                    format!("/{}", stripped.trim_start_matches('/'))
                } else {
                    stripped.to_string()
                };
                let new_path = if new_path.is_empty() { "/".to_string() } else { new_path };
                
                let path_and_query = match original_uri.query() {
                    Some(q) => format!("{}?{}", new_path, q),
                    None => new_path,
                };
                
                match http::Uri::builder().path_and_query(path_and_query.as_str()).build() {
                    Ok(uri) => upstream_request.set_uri(uri),
                    Err(e) => {
                        warn!(
                            req_id = %ctx.request_id,
                            original_path = %path,
                            attempted = %path_and_query,
                            error = %e,
                            "Failed to build URI, using root"
                        );
                        if let Ok(uri) = http::Uri::builder().path_and_query("/").build() {
                            upstream_request.set_uri(uri);
                        }
                    }
                }
            }
        }

        upstream_request.insert_header("X-Request-Id", &ctx.request_id)?;
        if let Some(user_id) = &ctx.user_id {
            upstream_request.insert_header("X-User-Id", user_id)?;
        }
        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let status = upstream_response.status.as_u16();
        info!(
            req_id = %ctx.request_id,
            status = %status,
            "Response"
        );
        
        if ctx.should_refresh {
            upstream_response.insert_header("X-Token-Refresh", "true")?;
        }

        upstream_response.insert_header("Access-Control-Allow-Origin", "*")?;
        upstream_response.insert_header("Access-Control-Expose-Headers", "X-Token-Refresh")?;

        Ok(())
    }
}
