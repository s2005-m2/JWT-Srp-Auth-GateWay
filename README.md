# ARC Auth Gateway

[中文文档](docs/app/cn_Zh_README.md) | English

High-performance authentication gateway built on Cloudflare Pingora + Axum, providing JWT-based authentication, user email registration/login/management and API gateway/reverse proxy/rate limiting capabilities.

All API and WebSocket/SSE requests are verified via JWT (Access Token + Refresh Token) at the gateway layer before reaching upstream services.

Designed to protect algorithm services behind an authentication gateway.

## Quick Deploy (Docker)

```bash
docker pull whereslow/arc_auth:latest
docker-compose up -d
```

Services:
- Gateway: http://localhost:8080
- Admin: http://localhost:3002

## Features

- **High-Performance Proxy**: Built on Pingora (Cloudflare's production-grade proxy framework)
- **JWT Authentication**: 24-hour Access Token + 7-day Refresh Token
- **Auto Refresh**: Automatic token refresh before expiration, seamless renewal
- **Email Registration**: Verification code registration flow
- **Dynamic Routing**: Configure proxy routes via admin dashboard
- **Static Routing**: Support for environment variables/config file routing, higher priority than database
- **Security Design**: SRP zero-knowledge password proof, rate limiting, Token Hash storage, header forgery protection
- **WebSocket/SSE**: Long connection proxy support, one-time authentication on connection establishment
- **API Keys**: 256-bit keys for external integrations and third-party applications

## System Architecture

```
Client -> Pingora Gateway (:8080) -> /auth/*  -> Axum Auth API (:3001) -> PostgreSQL
                                  -> /api/*   -> JWT Verification -> Upstream Services
                                  -> /ws/*    -> JWT Verification -> Upstream Services
```

## Quick Start

### Requirements

- Rust 1.70+
- PostgreSQL 14+
- Linux or WSL (Pingora primarily supports Linux)

### 1. Configure Database

```bash
sudo -u postgres createdb arc_auth
```

### 2. Configure Project

```bash
cp config/default.toml config/local.toml
```

Edit `config/local.toml`:

```toml
[database]
url = "postgres://user:password@localhost:5432/arc_auth"
```

### 3. Run (from source)

```bash
cargo run
```

After service starts:
- Gateway: http://localhost:8080
- Auth API: http://127.0.0.1:3001 (internal)

## Configuration

| Config | Default | Description |
|--------|---------|-------------|
| `server.gateway_port` | 8080 | Gateway port |
| `server.api_port` | 3001 | Auth API port |
| `database.url` | - | PostgreSQL connection string |
| `jwt.access_token_ttl` | 86400 | Access Token TTL (seconds) |
| `jwt.refresh_token_ttl` | 604800 | Refresh Token TTL (seconds) |

> **Note**: JWT Secret and SMTP configuration are managed in the database via admin dashboard, auto-generated on first startup.

### Static Route Configuration

Configure reverse proxy routes via config file or environment variables (higher priority than database dynamic routes):

**TOML Configuration:**

```toml
[[routing.routes]]
path = "/api/v1"
upstream = "127.0.0.1:8000"
auth = true

[[routing.routes]]
path = "/public"
upstream = "127.0.0.1:8001"
auth = false
```

**Environment Variables:**

```bash
ARC_AUTH__ROUTING__ROUTES__0__PATH=/api/v1
ARC_AUTH__ROUTING__ROUTES__0__UPSTREAM=127.0.0.1:8000
ARC_AUTH__ROUTING__ROUTES__0__AUTH=true

ARC_AUTH__ROUTING__ROUTES__1__PATH=/public
ARC_AUTH__ROUTING__ROUTES__1__UPSTREAM=127.0.0.1:8001
ARC_AUTH__ROUTING__ROUTES__1__AUTH=false
```

| Field | Description |
|-------|-------------|
| `path` | Path prefix matching |
| `upstream` | Upstream service address (host:port) |
| `auth` | Whether JWT authentication is required (default false) |

## API Documentation

For detailed integration documentation, see [docs/auth-integration.md](docs/auth-integration.md).

### Registration (SRP)

```http
POST /auth/register
{"email": "user@example.com"}
```

```http
POST /auth/register/verify
{"email": "user@example.com", "code": "123456", "salt": "<hex>", "verifier": "<hex>"}
```

### Login (SRP Two-Step Verification)

```http
POST /auth/login/init
{"email": "user@example.com", "client_public": "<hex>"}
```

```http
POST /auth/login/verify
{"session_id": "<uuid>", "client_proof": "<hex>"}
```

### Refresh Token

```http
POST /auth/refresh
{"refresh_token": "eyJ..."}
```

### Password Reset (SRP)

```http
POST /auth/password/reset
{"email": "user@example.com"}
```

```http
POST /auth/password/reset/confirm
{"email": "user@example.com", "code": "123456", "salt": "<hex>", "verifier": "<hex>"}
```

### Protected API

```http
GET /api/your-endpoint
Authorization: Bearer <access_token>
```

Gateway verifies JWT and injects `X-User-Id` header to upstream services.

### WebSocket

```javascript
const ws = new WebSocket('ws://localhost:8080/ws/your-endpoint', {
  headers: { 'Authorization': 'Bearer <access_token>' }
});
```

### SSE

```javascript
const es = new EventSource('/sse/your-endpoint', {
  headers: { 'Authorization': 'Bearer <access_token>' }
});
```

WebSocket and SSE connections perform JWT verification once on connection establishment, no repeated authentication during the connection.

## Error Codes

| Error Code | HTTP | Description |
|------------|------|-------------|
| `INVALID_EMAIL` | 400 | Invalid email format |
| `INVALID_CODE` | 400 | Invalid verification code |
| `WEAK_PASSWORD` | 400 | Password strength insufficient |
| `INVALID_CREDENTIALS` | 401 | Authentication failed |
| `INVALID_TOKEN` | 401 | Invalid token |
| `TOKEN_EXPIRED` | 401 | Token expired |
| `EMAIL_NOT_VERIFIED` | 403 | Email not verified |
| `EMAIL_EXISTS` | 409 | Email already exists |
| `RATE_LIMITED` | 429 | Request rate limit exceeded |
| `RESERVED_HEADER` | 400 | Request contains reserved headers (X-User-Id/X-Request-Id) |
| `NOT_FOUND` | 404 | Resource not found |

## API Keys

256-bit keys for external integrations and third-party applications.

### Creating Keys

Create via admin dashboard `/api-keys` page, or call the API:

```http
POST /api/config/api-keys
Authorization: Bearer <admin_token>
{"name": "My Integration", "permissions": ["*"]}
```

Response contains a 64-character hex key (shown only once):

```json
{
  "api_key": {"id": "...", "name": "My Integration", "key_prefix": "a1b2c3d4"},
  "raw_key": "a1b2c3d4e5f6..."
}
```

### Using Keys

External applications access via `X-API-Key` header:

```http
GET /api/some-endpoint
X-API-Key: a1b2c3d4e5f6...
```

### Permissions

| Permission | Description |
|------------|-------------|
| `*` | Full access |
| `routes:read` | Read route configuration |
| `routes:write` | Modify route configuration |
| `users:read` | Read user list |
| `stats:read` | Read statistics |

## Development

```bash
cargo check      # Compile check
cargo fmt        # Format
cargo clippy     # Lint
cargo test       # Test
```

## License

MIT
