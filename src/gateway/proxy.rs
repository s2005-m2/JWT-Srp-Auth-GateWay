use async_trait::async_trait;
use http::header::HeaderValue;
use pingora::http::{RequestHeader, ResponseHeader};
use pingora::prelude::HttpPeer;
use pingora::proxy::{ProxyHttp, Session};
use std::sync::Arc;

use super::config_cache::{ProxyConfigCache, MatchedRoute};
use super::jwt::{JwtError, JwtValidator};

type Result<T> = pingora::Result<T>;

impl AuthGateway {
    async fn send_error(&self, session: &mut Session, status: u16, msg: &str) -> Result<bool> {
        let mut header = ResponseHeader::build(status, None)?;
        header.insert_header("Content-Type", "application/json")?;
        let body = format!(r#"{{"error":{{"code":"{}","message":"{}"}}}}"#, status, msg);
        session.write_response_header(Box::new(header), false).await?;
        session.write_response_body(Some(body.into()), true).await?;
        Ok(true)
    }
}

pub struct AuthGateway {
    jwt_validator: Arc<JwtValidator>,
    config_cache: Arc<ProxyConfigCache>,
}

pub struct RequestCtx {
    pub user_id: Option<String>,
    pub should_refresh: bool,
    pub matched_route: Option<MatchedRoute>,
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
}

#[async_trait]
impl ProxyHttp for AuthGateway {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        RequestCtx {
            user_id: None,
            should_refresh: false,
            matched_route: None,
        }
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        let path = session.req_header().uri.path();

        let matched = match self.config_cache.match_route(path) {
            Some(r) => r,
            None => return self.send_error(session, 404, "Not found").await,
        };

        if matched.require_auth {
            let token = match Self::extract_bearer_token(session.req_header()) {
                Some(t) => t,
                None => return self.send_error(session, 401, "Missing token").await,
            };

            let claims = match self.jwt_validator.validate(token) {
                Ok(c) => c,
                Err(JwtError::Expired) => {
                    return self.send_error(session, 401, "Token expired").await;
                }
                Err(JwtError::Invalid) => {
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
        if let Some(user_id) = &ctx.user_id {
            upstream_request.insert_header("X-User-Id", user_id)?;
        }
        Ok(())
    }
}
