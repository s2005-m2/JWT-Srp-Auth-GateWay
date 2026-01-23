# ARC Auth Gateway

高性能鉴权网关，基于 Cloudflare Pingora + Axum 构建，为 arc-generater 提供用户认证和 API 网关能力。

## 特性

- **高性能代理**: 基于 Pingora (Cloudflare 生产级代理框架)
- **JWT 鉴权**: 24 小时 Access Token + 7 天 Refresh Token
- **自动刷新**: Token 即将过期时自动刷新，无感续期
- **邮箱注册**: 验证码注册流程
- **安全设计**: Argon2 密码哈希、速率限制、Token Hash 存储

## 系统架构

```
Client -> Pingora Gateway (:8080) -> /auth/*  -> Axum Auth API (:3001) -> PostgreSQL
                                  -> /api/*   -> JWT 验证 -> arc-generater (:7000)
                                  -> /ws/*    -> JWT 验证 -> arc-generater (:7000)
```

## 快速开始

### 环境要求

- Rust 1.70+
- PostgreSQL 14+
- Linux 或 WSL (Pingora 主要支持 Linux)

### 1. 安装依赖

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 PostgreSQL (Ubuntu/Debian)
sudo apt install postgresql postgresql-contrib
```

### 2. 配置数据库

```bash
# 创建数据库
sudo -u postgres createdb arc_auth

# 或使用 psql
sudo -u postgres psql -c "CREATE DATABASE arc_auth;"
```

### 3. 配置项目

复制并修改配置文件：

```bash
cp config/default.toml config/local.toml
```

编辑 `config/local.toml`：

```toml
[database]
url = "postgres://your_user:your_password@localhost:5432/arc_auth"

[jwt]
secret = "your-secure-secret-key-at-least-32-chars"

[email]
smtp_host = "smtp.your-provider.com"
smtp_port = 587
smtp_user = "your-email@example.com"
smtp_pass = "your-smtp-password"
from_email = "noreply@your-domain.com"
from_name = "ARC Platform"
```

### 4. 运行

```bash
# 开发模式
cargo run

# 生产构建
cargo build --release
./target/release/arc_auth
```

服务启动后：
- Gateway: http://localhost:8080
- Auth API: http://127.0.0.1:3001 (内部)

## 配置说明

### 环境变量覆盖

所有配置项都可通过环境变量覆盖，格式：`ARC_AUTH__<SECTION>__<KEY>`

```bash
# 示例
ARC_AUTH__JWT__SECRET="production-secret" cargo run
ARC_AUTH__DATABASE__URL="postgres://..." cargo run
```

### 配置项一览

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `server.gateway_port` | 8080 | Gateway 监听端口 |
| `server.api_port` | 3001 | Auth API 内部端口 |
| `database.url` | - | PostgreSQL 连接字符串 |
| `database.max_connections` | 10 | 连接池大小 |
| `jwt.secret` | - | JWT 签名密钥 (必填) |
| `jwt.access_token_ttl` | 86400 | Access Token 有效期 (秒) |
| `jwt.refresh_token_ttl` | 604800 | Refresh Token 有效期 (秒) |
| `email.smtp_host` | - | SMTP 服务器地址 |
| `email.smtp_port` | 587 | SMTP 端口 |

## API 文档

### 认证端点

#### 注册 - 发送验证码

```http
POST /auth/register
Content-Type: application/json

{
  "email": "user@example.com"
}
```

#### 注册 - 验证并创建账户

```http
POST /auth/register/verify
Content-Type: application/json

{
  "email": "user@example.com",
  "code": "123456",
  "password": "SecurePass123!"
}
```

响应：
```json
{
  "user": {
    "id": "uuid",
    "email": "user@example.com"
  },
  "access_token": "eyJ...",
  "refresh_token": "eyJ..."
}
```

#### 登录

```http
POST /auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePass123!"
}
```

#### 刷新 Token

```http
POST /auth/refresh
Content-Type: application/json

{
  "refresh_token": "eyJ..."
}
```

### 受保护 API 调用

```http
GET /api/your-endpoint
Authorization: Bearer <access_token>
```

Gateway 会自动验证 JWT 并注入 `X-User-Id` 头到上游服务。

### 错误响应格式

```json
{
  "error": {
    "code": "INVALID_CREDENTIALS",
    "message": "Invalid credentials"
  },
  "request_id": null
}
```

| 错误码 | HTTP 状态 | 说明 |
|--------|-----------|------|
| `INVALID_EMAIL` | 400 | 邮箱格式无效 |
| `INVALID_CODE` | 400 | 验证码错误或过期 |
| `WEAK_PASSWORD` | 400 | 密码强度不足 |
| `INVALID_CREDENTIALS` | 401 | 认证失败 |
| `INVALID_TOKEN` | 401 | Token 无效 |
| `TOKEN_EXPIRED` | 401 | Token 已过期 |
| `EMAIL_EXISTS` | 409 | 邮箱已存在 |
| `RATE_LIMITED` | 429 | 请求频率超限 |

## 密码要求

- 最少 8 字符
- 至少 1 个大写字母
- 至少 1 个小写字母
- 至少 1 个数字
- 至少 1 个特殊字符 (!@#$%^&*)

## 开发

```bash
# 编译检查
cargo check

# 代码格式化
cargo fmt

# Lint 检查
cargo clippy -- -D warnings

# 运行测试
cargo test
```

## 项目结构

```
arc_auth/
├── src/
│   ├── main.rs           # 入口
│   ├── config.rs         # 配置加载
│   ├── error.rs          # 错误处理
│   ├── api/              # Axum Auth API
│   │   ├── handlers/     # 请求处理器
│   │   └── middleware.rs # 中间件
│   ├── gateway/          # Pingora 代理
│   │   ├── proxy.rs      # 代理实现
│   │   └── jwt.rs        # JWT 验证
│   ├── services/         # 业务逻辑
│   └── models/           # 数据模型
├── migrations/           # 数据库迁移
└── config/               # 配置文件
```

## License

MIT
