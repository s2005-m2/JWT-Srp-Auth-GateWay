> 📍 Subsystem docs, see [root AGENTS.md](../AGENTS.md) for full architecture

# ARC_AUTH KNOWLEDGE BASE

**Generated:** 2026-02-09 | **Commit:** cb15430 | **Branch:** master

## OVERVIEW

High-performance authentication gateway. Triple-service architecture:
- Pingora reverse proxy (port 8080) - public gateway
- Axum Auth API (port 3001) - internal, proxied via gateway
- Axum Admin API (port 3002) - separate, can be disabled

Rust + PostgreSQL + JWT + SRP.

## COMMANDS

```bash
# Build
cargo build
cargo build --release

# Run
cargo run

# Check (fast compile check)
cargo check

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt
cargo fmt --check  # CI check

# Test
cargo test
cargo test <test_name>           # Single test
cargo test <module>::            # All tests in module
cargo test -- --nocapture        # Show println output
```

## STRUCTURE

```
arc_auth/
├── src/
│   ├── main.rs           # Entry: spawns Axum task, runs Pingora forever
│   ├── config.rs         # Config structs + loader (TOML + env)
│   ├── error.rs          # AppError enum + HTTP response mapping
│   ├── api/              # Axum auth endpoints
│   │   ├── mod.rs        # Router + AppState
│   │   ├── middleware.rs # RateLimiter (sliding window)
│   │   └── handlers/     # register, verify, srp_login, refresh, admin, api_key
│   ├── gateway/          # Pingora proxy
│   │   ├── proxy.rs      # AuthGateway ProxyHttp impl + header protection
│   │   ├── jwt.rs        # JwtValidator for gateway
│   │   └── config_cache.rs # Static + dynamic route merging
│   ├── services/         # Business logic (user, token, admin, api_key, srp, etc.)
│   └── models/           # User, VerificationCode, RefreshToken, ApiKey, Claims
├── migrations/
├── config/default.toml
└── web/                  # Admin frontend (React + Vite)
```

## CODE STYLE

### Imports (order matters)
```rust
// 1. std library
use std::sync::Arc;

// 2. External crates (alphabetical)
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// 3. Crate-internal (crate:: prefix)
use crate::error::{AppError, Result};

// 4. Super/self imports
use super::config_cache::ProxyConfigCache;
```

### Naming Conventions
| Item | Convention | Example |
|------|------------|---------|
| Structs | PascalCase | `UserService`, `LoginRequest` |
| Functions | snake_case | `find_by_email`, `verify_password` |
| Constants | SCREAMING_SNAKE | `MAX_CONNECTIONS` |
| Request/Response | `*Request`, `*Response` suffix | `LoginRequest`, `LoginResponse` |

### Error Handling
```rust
// Use crate::error::Result<T> (alias for Result<T, AppError>)
pub async fn login(...) -> Result<Json<LoginResponse>> {
    let user = state.user_service.find_by_email(&req.email).await?;
    return Err(AppError::InvalidCredentials);
}
```

### Handler Pattern
```rust
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    info!(email = %req.email, "Login attempt");
    Ok(Json(LoginResponse { ... }))
}
```

## ANTI-PATTERNS (NEVER DO)

| Rule | Reason |
|------|--------|
| Store raw refresh tokens | Only SHA256 hash in DB |
| Store plaintext passwords | Use SRP verifier only |
| Add routes without rate limiting | See `middleware.rs` pattern |
| Modify Pingora thread model | Axum spawned as tokio task, Pingora runs in main |
| Use `unwrap()` in handlers | Return `AppError` instead |
| Skip structured logging | Always use `tracing` macros with fields |
| Allow X-User-Id/X-Request-Id from client | Gateway rejects these (header spoofing protection) |

## WORKFLOW RULE: UNWRAP AUDIT

**Every task's final todo MUST be an `unwrap()` audit.** Before marking work complete:
1. Search all changed/new `.rs` files for `.unwrap()`, `.expect()`
2. Replace each with proper error handling (`?`, `map_err`, `unwrap_or_else`, `match`)
3. Add `tracing::warn!` or `tracing::error!` at each recovery point so failures leave a trace
4. Zero `unwrap()` in handler/service code — no exceptions

## ARCHITECTURE

```
Client -> Pingora (:8080) -> /auth/* -> Axum Auth (:3001) -> PostgreSQL
                          -> /api/*  -> JWT check -> upstream service

Admin  -> Axum Admin (:3002) -> /api/admin/* -> PostgreSQL
                             -> /api/config/*
                             -> /* (SPA)
```

**Key insight**: Gateway validates JWT but doesn't issue tokens. Auth API issues tokens. Admin API is separate and can be disabled.

**Route priority**: Static routes (env/config) > Dynamic routes (database) > Default upstream

## STATIC ROUTING (ENV/CONFIG)

```toml
[[routing.routes]]
path = "/api/v1"
upstream = "127.0.0.1:8000"
auth = true
```

Environment variables:
```bash
ARC_AUTH__ROUTING__ROUTES__0__PATH=/api/v1
ARC_AUTH__ROUTING__ROUTES__0__UPSTREAM=127.0.0.1:8000
ARC_AUTH__ROUTING__ROUTES__0__AUTH=true
```

## WHERE TO LOOK

| Task | Location |
|------|----------|
| Add auth endpoint | `src/api/handlers/` + route in `api/mod.rs` |
| Modify JWT validation | `src/gateway/jwt.rs` |
| Change token generation | `src/services/token.rs` |
| Add DB table | `migrations/` + models in `src/models/` |
| Proxy routing logic | `src/gateway/proxy.rs` |
| Static route config | `src/config.rs` + `config/default.toml` |
| Rate limiting | `src/api/middleware.rs` |
| API Keys management | `src/services/api_key.rs` + `src/api/handlers/api_key.rs` |
| SRP authentication | `src/services/srp.rs` + `src/api/handlers/srp_login.rs` |

## PORTS

| Service | Port | Binding | Purpose |
|---------|------|---------|---------|
| Pingora Gateway | 8080 | 0.0.0.0 (public) | Auth proxy + API gateway |
| Axum Auth API | 3001 | 127.0.0.1 (internal) | Login/register (via gateway) |
| Axum Admin API | 3002 | 0.0.0.0 (optional) | Admin panel (can be disabled) |
| PostgreSQL | 5432 | localhost | Database |

## SECURITY

- **SRP Authentication**: Zero-knowledge password proof (SRP-6a), server never sees plaintext password
- **Header protection**: Gateway rejects requests containing `X-User-Id` or `X-Request-Id` headers (prevents spoofing)
- **Token storage**: SHA256 hash of refresh tokens and API keys in DB
- **Rate limiting**: Sliding window per endpoint
- **API Keys**: 256-bit keys for external integrations, no expiration, permission-scoped

## API KEYS

External integration keys for third-party applications:

| Field | Description |
|-------|-------------|
| `key_hash` | SHA256 hash (raw key never stored) |
| `key_prefix` | First 8 chars for identification |
| `permissions` | JSON array: `["*"]`, `["routes:read", "stats:read"]` |

**Endpoints** (Admin API, requires admin JWT):
- `GET /api/config/api-keys` - List keys
- `POST /api/config/api-keys` - Create key (returns raw key once)
- `DELETE /api/config/api-keys/:id` - Delete key

**Usage**: External apps use `X-API-Key: <64-char-hex>` header.

## NOTES

- **Platform**: Pingora primarily supports Linux. Windows dev requires WSL.
- **No rustfmt.toml**: Uses default rustfmt settings.
- **Edition**: Rust 2021
- **Migrations**: Run automatically on startup via `sqlx::migrate!`
