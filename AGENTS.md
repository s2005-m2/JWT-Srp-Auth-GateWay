# ARC_AUTH KNOWLEDGE BASE

**Generated:** 2026-01-23

## OVERVIEW

High-performance auth gateway for arc-generater. Dual-service architecture: Pingora reverse proxy (port 8080) + Axum auth API (port 3001 internal). Rust + PostgreSQL + JWT.

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

# Test (when tests exist)
cargo test
cargo test <test_name>           # Single test
cargo test <module>::             # All tests in module
cargo test -- --nocapture        # Show println output

# Database
# Migrations run automatically on startup via sqlx::migrate!

# Config override
ARC_AUTH__JWT__SECRET="prod-secret" cargo run
```

## STRUCTURE

```
arc_auth/
├── src/
│   ├── main.rs           # Entry: spawns Axum task, runs Pingora forever
│   ├── lib.rs            # Module exports
│   ├── config.rs         # Config structs + loader (TOML + env)
│   ├── error.rs          # AppError enum + HTTP response mapping
│   ├── api/              # Axum auth endpoints
│   │   ├── mod.rs        # Router + AppState
│   │   ├── middleware.rs # RateLimiter (sliding window)
│   │   └── handlers/     # register, verify, login, refresh, admin
│   ├── gateway/          # Pingora proxy
│   │   ├── mod.rs
│   │   ├── proxy.rs      # AuthGateway ProxyHttp impl
│   │   ├── jwt.rs        # JwtValidator for gateway
│   │   └── config_cache.rs
│   ├── services/         # Business logic
│   │   ├── user.rs       # UserService + Argon2 password
│   │   ├── token.rs      # TokenService + JWT generation
│   │   ├── email.rs      # EmailService + SMTP
│   │   ├── admin.rs      # AdminService
│   │   └── proxy_config.rs
│   ├── models/           # User, VerificationCode, RefreshToken, Claims
│   └── db/               # PgPool + migrations
├── migrations/
├── config/
│   └── default.toml
└── design.md             # Full design doc (Chinese)
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
use crate::api::AppState;
use crate::error::{AppError, Result};
use crate::models::UserInfo;

// 4. Super/self imports
use super::config_cache::{ProxyConfigCache, MatchedRoute};
```

### Naming Conventions
| Item | Convention | Example |
|------|------------|---------|
| Structs | PascalCase | `UserService`, `LoginRequest` |
| Functions | snake_case | `find_by_email`, `verify_password` |
| Constants | SCREAMING_SNAKE | `MAX_CONNECTIONS` |
| Type aliases | PascalCase | `type Result<T> = std::result::Result<T, AppError>` |
| Request/Response | `*Request`, `*Response` suffix | `LoginRequest`, `LoginResponse` |

### Error Handling
```rust
// Use crate::error::Result<T> (alias for Result<T, AppError>)
pub async fn login(...) -> Result<Json<LoginResponse>> {
    // Use ? for propagation
    let user = state.user_service.find_by_email(&req.email).await?;
    
    // Return specific AppError variants
    return Err(AppError::InvalidCredentials);
}

// Add new errors to src/error.rs AppError enum
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    // ...
}
```

### Async Patterns
```rust
// Services wrap Arc<PgPool>
pub struct UserService {
    pool: Arc<PgPool>,
}

impl UserService {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(self.pool.as_ref())
            .await?;
        Ok(user)
    }
}
```

### Handler Pattern
```rust
// Request/Response structs with derive macros
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user: UserInfo,
    pub access_token: String,
}

// Handler signature: State + Json extractors -> Result<Json<Response>>
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    // Structured logging
    info!(email = %req.email, "Login attempt");
    warn!(email = %req.email, "Login failed: user not found");
    
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

## ARCHITECTURE

```
Client -> Pingora (:8080) -> /auth/* -> Axum (:3001) -> PostgreSQL
                          -> /api/*  -> JWT check -> arc-generater (:7000)
                          -> /ws/*   -> JWT check -> arc-generater (:7000)
```

**Key insight**: Gateway validates JWT but doesn't issue tokens. Axum API issues tokens. They share JWT secret via config.

## WHERE TO LOOK

| Task | Location |
|------|----------|
| Add auth endpoint | `src/api/handlers/` + route in `api/mod.rs` |
| Modify JWT validation | `src/gateway/jwt.rs` |
| Change token generation | `src/services/token.rs` |
| Add DB table | `migrations/` + models in `src/models/` |
| Proxy routing logic | `src/gateway/proxy.rs` |
| Rate limiting | `src/api/middleware.rs` |
| Config options | `src/config.rs` + `config/default.toml` |

## PORTS

| Service | Port | Binding |
|---------|------|---------|
| Pingora Gateway | 8080 | 0.0.0.0 (public) |
| Axum Auth API | 3001 | 127.0.0.1 (internal) |
| PostgreSQL | 5432 | localhost |
| arc-generater | 7000 | localhost |

## NOTES

- **Platform**: Pingora primarily supports Linux. Windows dev requires WSL.
- **No rustfmt.toml**: Uses default rustfmt settings.
- **No tests yet**: `tokio-test` in dev-deps but no test files.
- **Edition**: Rust 2021
