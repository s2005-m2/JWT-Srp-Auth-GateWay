# ARC_AUTH KNOWLEDGE BASE

**Generated:** 2026-01-23

## OVERVIEW

High-performance auth gateway for arc-generater. Dual-service architecture: Pingora reverse proxy (port 8080) + Axum auth API (port 3001 internal). Rust + PostgreSQL + JWT.

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
│   │   └── handlers/     # register, verify, login, refresh
│   ├── gateway/          # Pingora proxy
│   │   ├── mod.rs
│   │   ├── proxy.rs      # AuthGateway ProxyHttp impl
│   │   └── jwt.rs        # JwtValidator for gateway
│   ├── services/         # Business logic
│   │   ├── user.rs       # UserService + Argon2 password
│   │   ├── token.rs      # TokenService + JWT generation
│   │   └── email.rs      # EmailService + SMTP
│   ├── models/           # User, VerificationCode, RefreshToken, Claims
│   └── db/               # PgPool + migrations
├── migrations/
│   └── 001_init.sql      # users, verification_codes, refresh_tokens
├── config/
│   └── default.toml      # Default config (ports, DB, JWT, email)
└── design.md             # Comprehensive design doc (Chinese)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add auth endpoint | `src/api/handlers/` | Create handler, add route in `api/mod.rs` |
| Modify JWT validation | `src/gateway/jwt.rs` | Gateway-side validation |
| Change token generation | `src/services/token.rs` | API-side token creation |
| Add DB table | `migrations/` | New .sql file, models in `src/models/` |
| Proxy routing logic | `src/gateway/proxy.rs` | `request_filter` method |
| Rate limiting | `src/api/middleware.rs` | RateLimiter struct |
| Config options | `src/config.rs` + `config/default.toml` | Add struct field + TOML key |

## ARCHITECTURE

```
Client → Pingora (:8080) → /auth/* → Axum (:3001) → PostgreSQL
                        → /api/*  → JWT check → arc-generater (:7000)
                        → /ws/*   → JWT check → arc-generater (:7000)
```

**Key insight**: Gateway validates JWT but doesn't issue tokens. Axum API issues tokens. They share JWT secret via config.

## CONVENTIONS

| Rule | Details |
|------|---------|
| Error handling | `AppError` enum → `thiserror` + `IntoResponse` |
| Password hashing | Argon2 (never bcrypt) |
| JWT algorithm | HS256 with shared secret |
| Token storage | Refresh token hash only (SHA256) |
| Config loading | `config/default.toml` → `config/local.toml` → `ARC_AUTH__*` env |
| Async runtime | Tokio (Axum) + Pingora's internal runtime |

## ANTI-PATTERNS (THIS PROJECT)

| Rule | Reason |
|------|--------|
| **NEVER** store raw refresh tokens | Only SHA256 hash in DB |
| **NEVER** use bcrypt | Project uses Argon2 exclusively |
| **NEVER** add routes without rate limiting | See `middleware.rs` pattern |
| **DO NOT** modify Pingora thread model | Axum spawned as tokio task, Pingora runs in main |

## PORTS

| Service | Port | Binding |
|---------|------|---------|
| Pingora Gateway | 8080 | 0.0.0.0 (public) |
| Axum Auth API | 3001 | 127.0.0.1 (internal only) |
| PostgreSQL | 5432 | localhost |
| arc-generater (upstream) | 7000 | localhost |

## COMMANDS

```bash
# Development
cargo build
cargo run

# Database
# Migrations run automatically on startup via sqlx::migrate!

# Config override
ARC_AUTH__JWT__SECRET="prod-secret" cargo run
```

## DEPENDENCIES (KEY)

| Crate | Purpose |
|-------|---------|
| pingora | Cloudflare's proxy framework |
| axum | Auth API web framework |
| sqlx | Async PostgreSQL (compile-time checked) |
| jsonwebtoken | JWT encode/decode |
| argon2 | Password hashing |
| mail-send | SMTP client |

## NOTES

- **Platform**: Pingora primarily supports Linux. Windows dev requires WSL.
- **design.md**: Contains full API specs, data models, flow diagrams (Chinese).
- **Token refresh**: Gateway sets `X-New-Access-Token` header when token near expiry.
- **Email templates**: Hardcoded HTML in `services/email.rs` (Chinese text).
- **No tests yet**: `tokio-test` in dev-deps but no test files.
