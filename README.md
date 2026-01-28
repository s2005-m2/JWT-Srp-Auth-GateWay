# ARC Auth Gateway

高性能鉴权网关，基于 Cloudflare Pingora + Axum 构建，提供用户认证和 API 网关能力。

## 特性

- **高性能代理**: 基于 Pingora (Cloudflare 生产级代理框架)
- **JWT 鉴权**: 24 小时 Access Token + 7 天 Refresh Token
- **自动刷新**: Token 即将过期时自动刷新，无感续期
- **邮箱注册**: 验证码注册流程
- **动态路由**: 通过管理后台配置代理路由
- **静态路由**: 支持环境变量/配置文件配置路由，优先级高于数据库
- **安全设计**: SRP 零知识密码证明、速率限制、Token Hash 存储、Header 伪造防护
- **WebSocket/SSE**: 支持长连接代理，连接建立时一次性鉴权
- **API Keys**: 256 位密钥，用于外部集成和第三方应用

## 系统架构

```
Client -> Pingora Gateway (:8080) -> /auth/*  -> Axum Auth API (:3001) -> PostgreSQL
                                  -> /api/*   -> JWT 验证 -> 上游服务
                                  -> /ws/*    -> JWT 验证 -> 上游服务
```

## 快速开始

### 环境要求

- Rust 1.70+
- PostgreSQL 14+
- Linux 或 WSL (Pingora 主要支持 Linux)

### 1. 配置数据库

```bash
sudo -u postgres createdb arc_auth
```

### 2. 配置项目

```bash
cp config/default.toml config/local.toml
```

编辑 `config/local.toml`：

```toml
[database]
url = "postgres://user:password@localhost:5432/arc_auth"
```

### 3. 运行

```bash
cargo run
```

服务启动后：
- Gateway: http://localhost:8080
- Auth API: http://127.0.0.1:3001 (内部)

## 配置说明

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `server.gateway_port` | 8080 | Gateway 端口 |
| `server.api_port` | 3001 | Auth API 端口 |
| `database.url` | - | PostgreSQL 连接串 |
| `jwt.access_token_ttl` | 86400 | Access Token 有效期 (秒) |
| `jwt.refresh_token_ttl` | 604800 | Refresh Token 有效期 (秒) |

> **注意**: JWT Secret 和 SMTP 配置通过管理后台在数据库中管理，首次启动时自动生成。

### 静态路由配置

通过配置文件或环境变量配置反向代理路由（优先级高于数据库动态路由）：

**TOML 配置：**

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

**环境变量：**

```bash
ARC_AUTH__ROUTING__ROUTES__0__PATH=/api/v1
ARC_AUTH__ROUTING__ROUTES__0__UPSTREAM=127.0.0.1:8000
ARC_AUTH__ROUTING__ROUTES__0__AUTH=true

ARC_AUTH__ROUTING__ROUTES__1__PATH=/public
ARC_AUTH__ROUTING__ROUTES__1__UPSTREAM=127.0.0.1:8001
ARC_AUTH__ROUTING__ROUTES__1__AUTH=false
```

| 字段 | 说明 |
|------|------|
| `path` | 路径前缀匹配 |
| `upstream` | 上游服务地址 (host:port) |
| `auth` | 是否需要 JWT 鉴权 (默认 false) |

## API 文档

详细的接入文档请参考 [docs/auth-integration.md](docs/auth-integration.md)。

### 注册 (SRP)

```http
POST /auth/register
{"email": "user@example.com"}
```

```http
POST /auth/register/verify
{"email": "user@example.com", "code": "123456", "salt": "<hex>", "verifier": "<hex>"}
```

### 登录 (SRP 两步验证)

```http
POST /auth/login/init
{"email": "user@example.com", "client_public": "<hex>"}
```

```http
POST /auth/login/verify
{"session_id": "<uuid>", "client_proof": "<hex>"}
```

### 刷新 Token

```http
POST /auth/refresh
{"refresh_token": "eyJ..."}
```

### 密码重置 (SRP)

```http
POST /auth/password/reset
{"email": "user@example.com"}
```

```http
POST /auth/password/reset/confirm
{"email": "user@example.com", "code": "123456", "salt": "<hex>", "verifier": "<hex>"}
```

### 受保护 API

```http
GET /api/your-endpoint
Authorization: Bearer <access_token>
```

Gateway 验证 JWT 后注入 `X-User-Id` 头到上游服务。

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

WebSocket 和 SSE 连接在建立时进行一次 JWT 验证，连接期间不再重复鉴权。

## 错误码

| 错误码 | HTTP | 说明 |
|--------|------|------|
| `INVALID_EMAIL` | 400 | 邮箱格式无效 |
| `INVALID_CODE` | 400 | 验证码错误 |
| `WEAK_PASSWORD` | 400 | 密码强度不足 |
| `INVALID_CREDENTIALS` | 401 | 认证失败 |
| `INVALID_TOKEN` | 401 | Token 无效 |
| `TOKEN_EXPIRED` | 401 | Token 过期 |
| `EMAIL_NOT_VERIFIED` | 403 | 邮箱未验证 |
| `EMAIL_EXISTS` | 409 | 邮箱已存在 |
| `RATE_LIMITED` | 429 | 请求频率超限 |
| `RESERVED_HEADER` | 400 | 请求包含保留 Header (X-User-Id/X-Request-Id) |
| `NOT_FOUND` | 404 | 资源不存在 |

## API Keys

用于外部集成和第三方应用的 256 位密钥。

### 创建密钥

通过管理后台 `/api-keys` 页面创建，或调用 API：

```http
POST /api/config/api-keys
Authorization: Bearer <admin_token>
{"name": "My Integration", "permissions": ["*"]}
```

响应包含 64 字符的 hex 密钥（仅显示一次）：

```json
{
  "api_key": {"id": "...", "name": "My Integration", "key_prefix": "a1b2c3d4"},
  "raw_key": "a1b2c3d4e5f6..."
}
```

### 使用密钥

外部应用通过 `X-API-Key` header 访问：

```http
GET /api/some-endpoint
X-API-Key: a1b2c3d4e5f6...
```

### 权限

| 权限 | 说明 |
|------|------|
| `*` | 完全访问 |
| `routes:read` | 读取路由配置 |
| `routes:write` | 修改路由配置 |
| `users:read` | 读取用户列表 |
| `stats:read` | 读取统计数据 |

## 开发

```bash
cargo check      # 编译检查
cargo fmt        # 格式化
cargo clippy     # Lint
cargo test       # 测试
```

## License

MIT
