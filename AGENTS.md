> 📍 子系统文档，全局架构见 [根 AGENTS.md](../AGENTS.md)

# ARC_AUTH KNOWLEDGE BASE

**Updated:** 2026-01-24

## OVERVIEW

High-performance authentication gateway. Dual-service architecture: Pingora reverse proxy (port 8080) + Axum auth API (port 3001 internal). Rust + PostgreSQL + JWT.

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
│   │   └── handlers/     # register, verify, login, refresh, admin
│   ├── gateway/          # Pingora proxy
│   │   ├── proxy.rs      # AuthGateway ProxyHttp impl + header protection
│   │   ├── jwt.rs        # JwtValidator for gateway
│   │   └── config_cache.rs # Static + dynamic route merging
│   ├── services/         # Business logic
│   └── models/           # User, VerificationCode, RefreshToken, Claims
├── migrations/
└── config/default.toml
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
| Use bcrypt | Project uses Argon2 exclusively |
| Add routes without rate limiting | See `middleware.rs` pattern |
| Modify Pingora thread model | Axum spawned as tokio task, Pingora runs in main |
| Use `unwrap()` in handlers | Return `AppError` instead |
| Skip structured logging | Always use `tracing` macros with fields |
| Allow X-User-Id/X-Request-Id from client | Gateway rejects these (header spoofing protection) |

## ARCHITECTURE

```
Client -> Pingora (:8080) -> /auth/* -> Axum (:3001) -> PostgreSQL
                          -> /api/*  -> JWT check -> upstream service
```

**Key insight**: Gateway validates JWT but doesn't issue tokens. Axum API issues tokens.

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

## PORTS

| Service | Port | Binding |
|---------|------|---------|
| Pingora Gateway | 8080 | 0.0.0.0 (public) |
| Axum Auth API | 3001 | 127.0.0.1 (internal) |
| PostgreSQL | 5432 | localhost |

## SECURITY

- **Header protection**: Gateway rejects requests containing `X-User-Id` or `X-Request-Id` headers (prevents spoofing)
- **Password hashing**: Argon2 only
- **Token storage**: SHA256 hash of refresh tokens in DB
- **Rate limiting**: Sliding window per endpoint

## NOTES

- **Platform**: Pingora primarily supports Linux. Windows dev requires WSL.
- **No rustfmt.toml**: Uses default rustfmt settings.
- **Edition**: Rust 2021
- **Migrations**: Run automatically on startup via `sqlx::migrate!`
