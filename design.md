# ARC Auth Gateway 设计文档

## 1. 概述

`arc_auth` 是一个高性能鉴权网关，基于 Cloudflare Pingora + Axum 构建，为 `arc-generater` 提供用户认证和 API 网关能力。

### 1.1 设计目标

| 目标 | 实现方式 |
|------|----------|
| 高性能 | Pingora 代理 (Cloudflare 生产验证) |
| 零内存泄漏 | Rust 所有权系统 + 无 `unsafe` |
| 邮箱注册 | Stalwart mail-builder + mail-send |
| JWT 鉴权 | 24h 过期 + 自动刷新 |

### 1.2 系统架构

```
                                    ┌─────────────────────────────────┐
                                    │          arc_auth               │
┌──────────┐     ┌──────────────────┴─────────────────────────────────┴──────────────────┐
│  Client  │────▶│  Pingora Gateway (:8080)                                              │
└──────────┘     │  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐   │
                 │  │  request_filter │───▶│  JWT Validator  │───▶│  upstream_peer  │   │
                 │  │  (路由分发)      │    │  (鉴权检查)      │    │  (转发上游)     │   │
                 │  └─────────────────┘    └─────────────────┘    └─────────────────┘   │
                 └───────────────────────────────────────────────────────────────────────┘
                          │                                              │
                          │ 公开路由                                      │ 受保护路由
                          ▼                                              ▼
                 ┌─────────────────┐                            ┌─────────────────┐
                 │  Axum Auth API  │                            │  arc-generater  │
                 │  (:3001 内部)    │                            │  (:7000)        │
                 └─────────────────┘                            └─────────────────┘
                          │
                          ▼
                 ┌─────────────────┐
                 │   PostgreSQL    │
                 │   (用户数据)     │
                 └─────────────────┘
```

### 1.3 端口规划

| 服务 | 端口 | 说明 |
|------|------|------|
| Pingora Gateway | 8080 | 对外统一入口 |
| Axum Auth API | 3001 | 内部认证服务 |
| arc-generater | 7000 | 上游服务 (已有) |

---

## 2. 路由设计

### 2.1 路由分类

| 路径前缀 | 类型 | 处理方式 |
|----------|------|----------|
| `/auth/*` | 公开 | 转发到 Axum Auth API |
| `/api/*` | 受保护 | JWT 验证后转发 arc-generater |
| `/ws/*` | 受保护 | JWT 验证后转发 WebSocket |

### 2.2 Auth API 端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/auth/register` | POST | 发送注册验证码 |
| `/auth/register/verify` | POST | 验证码确认注册 |
| `/auth/login` | POST | 邮箱密码登录 |
| `/auth/refresh` | POST | 刷新 Token |
| `/auth/logout` | POST | 登出 (撤销 Token) |
| `/auth/password/reset` | POST | 发送重置密码邮件 |
| `/auth/password/confirm` | POST | 确认重置密码 |
| `/auth/me` | GET | 获取当前用户信息 (需 JWT) |

---

## 3. 数据模型

### 3.1 用户表 (users)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键，自动生成 |
| email | VARCHAR(255) | 唯一，邮箱地址 |
| password_hash | VARCHAR(255) | Argon2 哈希后的密码 |
| email_verified | BOOLEAN | 邮箱是否已验证 |
| created_at | TIMESTAMPTZ | 创建时间 |
| updated_at | TIMESTAMPTZ | 更新时间 |

索引：`email` 字段建立唯一索引

### 3.2 验证码表 (verification_codes)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| email | VARCHAR(255) | 目标邮箱 |
| code | VARCHAR(6) | 6 位数字验证码 |
| code_type | VARCHAR(20) | 类型：'register' 或 'reset_password' |
| expires_at | TIMESTAMPTZ | 过期时间 |
| used | BOOLEAN | 是否已使用 |
| created_at | TIMESTAMPTZ | 创建时间 |

索引：`(email, code_type)` 复合索引

### 3.3 刷新令牌表 (refresh_tokens)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| user_id | UUID | 外键关联 users，级联删除 |
| token_hash | VARCHAR(255) | Token 的 SHA256 哈希 |
| expires_at | TIMESTAMPTZ | 过期时间 |
| revoked | BOOLEAN | 是否已撤销 |
| created_at | TIMESTAMPTZ | 创建时间 |

索引：`user_id` 和 `token_hash` 分别建立索引

---

## 4. JWT 设计

### 4.1 Token 结构

**Access Token Claims:**
- `sub`: 用户 ID (UUID)
- `email`: 用户邮箱
- `exp`: 过期时间戳 (24 小时后)
- `iat`: 签发时间戳
- `jti`: Token 唯一 ID (用于黑名单)

**Refresh Token Claims:**
- `sub`: 用户 ID
- `exp`: 过期时间戳 (7 天后)
- `iat`: 签发时间戳
- `jti`: Token 唯一 ID

### 4.2 Token 生命周期

| Token 类型 | 有效期 | 存储位置 |
|------------|--------|----------|
| Access Token | 24 小时 | 客户端 (Authorization Header) |
| Refresh Token | 7 天 | 客户端 + 数据库 (仅存 Hash) |

### 4.3 自动刷新机制

```
时序流程:
1. 客户端发送请求，携带 JWT
2. Gateway 检查 JWT 剩余有效期
3. 若剩余时间 < 1 小时:
   a. Gateway 内部调用 Auth API 刷新
   b. 获取新 Access Token
   c. 在响应头添加 X-New-Access-Token
4. 客户端检测响应头，更新本地 Token
```

响应头: `X-New-Access-Token: <new_jwt>`

---

## 5. Pingora Gateway 设计

### 5.1 核心组件

**AuthGateway 结构:**
- `jwt_validator`: JWT 验证器 (共享引用)
- `auth_upstream`: Auth API 上游地址 (localhost:3001)
- `api_upstream`: arc-generater 上游地址 (localhost:7000)

**RequestCtx 请求上下文:**
- `user_id`: 解析出的用户 ID (可选)
- `should_refresh`: 是否需要刷新 Token
- `new_token`: 新生成的 Token (可选)

### 5.2 请求处理流程

```
伪代码:

fn request_filter(session, ctx):
    path = session.get_path()
    
    IF path.starts_with("/auth/"):
        // 公开路由，直接转发到 Auth API
        RETURN Ok(转发到 auth_upstream)
    
    IF path.starts_with("/api/") OR path.starts_with("/ws/"):
        // 受保护路由，需要验证 JWT
        token = extract_bearer_token(session.headers)
        
        IF token IS None:
            RETURN Error(401, "Missing token")
        
        claims = jwt_validator.validate(token)
        IF claims IS Error:
            RETURN Error(401, "Invalid token")
        
        // 检查是否需要自动刷新
        IF claims.exp - now() < 1_HOUR:
            ctx.should_refresh = true
        
        // 注入用户信息到请求头
        session.headers.insert("X-User-Id", claims.sub)
        ctx.user_id = claims.sub
        
        RETURN Ok(转发到 api_upstream)
    
    RETURN Error(404, "Not found")

fn response_filter(session, ctx):
    IF ctx.should_refresh AND ctx.new_token IS Some:
        session.response_headers.insert("X-New-Access-Token", ctx.new_token)
```

---

## 6. 项目结构

```
arc_auth/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口: 启动 Pingora + Axum
│   ├── config.rs            # 配置加载与管理
│   ├── gateway/
│   │   ├── mod.rs
│   │   ├── proxy.rs         # Pingora ProxyHttp 实现
│   │   └── jwt.rs           # JWT 验证逻辑
│   ├── api/
│   │   ├── mod.rs
│   │   ├── routes.rs        # Axum 路由定义
│   │   ├── handlers/        # 各端点处理器
│   │   └── middleware.rs    # 中间件
│   ├── services/
│   │   ├── user.rs          # 用户业务逻辑
│   │   ├── token.rs         # Token 生成/验证
│   │   └── email.rs         # 邮件发送
│   ├── models/              # 数据模型定义
│   ├── db/                  # 数据库连接与仓储
│   └── error.rs             # 统一错误处理
├── migrations/
│   └── 001_init.sql         # 数据库迁移脚本
└── config/
    ├── default.toml         # 默认配置
    └── production.toml      # 生产配置
```

---

## 7. 核心服务设计

### 7.1 邮件服务

**职责:** 发送验证码邮件

**配置项:**
- SMTP 主机、端口、用户名、密码
- 发件人邮箱和名称

**主要方法:**
- `send_verification_code(to_email, code)`: 发送验证码
- `send_password_reset(to_email, code)`: 发送密码重置邮件

**依赖:** Stalwart mail-builder + mail-send

### 7.2 Token 服务

**职责:** JWT 生成、验证、刷新

**主要方法:**
- `generate_access_token(user_id, email)`: 生成 Access Token
- `generate_refresh_token(user_id)`: 生成 Refresh Token，返回 (token, hash)
- `validate_access_token(token)`: 验证并解析 Access Token
- `refresh(refresh_token)`: 使用 Refresh Token 获取新 Access Token

**安全措施:**
- 使用 HS256 算法签名
- Refresh Token 仅存储 SHA256 哈希到数据库

### 7.3 用户服务

**职责:** 用户 CRUD 操作

**主要方法:**
- `find_by_email(email)`: 按邮箱查找用户
- `create(email, password_hash)`: 创建新用户
- `update_password(user_id, new_hash)`: 更新密码
- `verify_email(user_id)`: 标记邮箱已验证

---

## 8. 安全措施

### 8.1 密码安全

- **哈希算法:** Argon2 (内存硬函数，抗 GPU 暴力破解)
- **盐值:** 每次哈希自动生成随机盐

### 8.2 验证码安全

| 规则 | 值 |
|------|-----|
| 长度 | 6 位数字 |
| 有效期 | 10 分钟 |
| 使用次数 | 单次 |
| 重发间隔 | 同邮箱 1 分钟内不可重发 |

### 8.3 密码强度要求

- 最少 8 字符
- 至少 1 个大写字母
- 至少 1 个小写字母
- 至少 1 个数字
- 至少 1 个特殊字符 (!@#$%^&*)

---

## 9. 速率限制

### 9.1 限制策略

| 端点 | 限制维度 | 限制值 | 窗口 |
|------|----------|--------|------|
| POST /auth/register | IP | 5 次 | 1 小时 |
| POST /auth/register | Email | 1 次 | 1 分钟 |
| POST /auth/login | IP | 10 次 | 1 分钟 |
| POST /auth/login | Email | 5 次 | 5 分钟 |
| POST /auth/password/reset | IP | 3 次 | 10 分钟 |
| POST /auth/password/reset | Email | 1 次 | 1 分钟 |
| POST /auth/refresh | User | 60 次 | 1 分钟 |

### 9.2 实现方案

**算法:** 滑动窗口

**存储:** 内存 HashMap (单实例部署)

```
伪代码:

struct RateLimiter:
    windows: Map<String, List<Timestamp>>
    max_requests: int
    window_duration: Duration

fn check(key):
    now = current_time()
    timestamps = windows.get_or_create(key)
    
    // 清理过期记录
    timestamps.retain(t => now - t < window_duration)
    
    IF timestamps.len() >= max_requests:
        RETURN Error(RateLimited)
    
    timestamps.push(now)
    RETURN Ok
```

**注意:** 多实例部署需改用 Redis

---

## 10. API 处理流程

### 10.1 注册流程

```
POST /auth/register:
1. 验证邮箱格式
2. 检查邮箱是否已注册 → 已注册返回 409
3. 检查速率限制 → 超限返回 429
4. 生成 6 位随机验证码
5. 存储验证码到数据库 (10 分钟过期)
6. 发送验证码邮件
7. 返回成功响应

POST /auth/register/verify:
1. 验证密码强度 → 不足返回 400
2. 查询验证码记录 (未过期、未使用)
3. 验证码不匹配 → 返回 400
4. 标记验证码已使用
5. 哈希密码，创建用户
6. 生成 Access Token + Refresh Token
7. 存储 Refresh Token Hash 到数据库
8. 返回用户信息和 Tokens
```

### 10.2 登录流程

```
POST /auth/login:
1. 检查速率限制
2. 按邮箱查找用户 → 不存在返回 401
3. 验证密码 → 不匹配返回 401
4. 生成 Access Token + Refresh Token
5. 存储 Refresh Token Hash
6. 返回用户信息和 Tokens
```

### 10.3 Token 刷新流程

```
POST /auth/refresh:
1. 解析 Refresh Token Claims
2. 计算 Token Hash
3. 查询数据库验证 Hash 存在且未撤销
4. 生成新 Access Token
5. 返回新 Token
```

### 10.4 密码重置流程

```
POST /auth/password/reset:
1. 检查速率限制
2. 按邮箱查找用户 (不存在也返回成功，防止枚举)
3. 生成验证码，存储到数据库
4. 发送重置邮件
5. 返回成功响应

POST /auth/password/confirm:
1. 验证新密码强度
2. 验证验证码
3. 哈希新密码，更新用户记录
4. 撤销该用户所有 Refresh Token
5. 返回成功响应
```

---

## 11. 错误处理

### 11.1 错误码清单

| 错误码 | HTTP 状态 | 说明 |
|--------|-----------|------|
| INVALID_EMAIL | 400 | 邮箱格式无效 |
| INVALID_CODE | 400 | 验证码错误或过期 |
| WEAK_PASSWORD | 400 | 密码强度不足 |
| INVALID_REQUEST | 400 | 请求参数错误 |
| INVALID_CREDENTIALS | 401 | 认证失败 |
| INVALID_TOKEN | 401 | Token 无效 |
| TOKEN_EXPIRED | 401 | Token 已过期 |
| TOKEN_REVOKED | 401 | Token 已撤销 |
| EMAIL_NOT_VERIFIED | 403 | 邮箱未验证 |
| EMAIL_EXISTS | 409 | 邮箱已存在 |
| RATE_LIMITED | 429 | 请求频率超限 |
| INTERNAL_ERROR | 500 | 服务器内部错误 |

### 11.2 统一错误响应格式

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "人类可读的错误描述",
    "details": null
  },
  "request_id": "req_xxx"
}
```

---

## 12. 配置项

### 12.1 服务器配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| server.gateway_port | 8080 | Gateway 监听端口 |
| server.api_port | 3001 | Auth API 监听端口 |

### 12.2 上游配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| upstream.arc_generater | 127.0.0.1:7000 | arc-generater 地址 |

### 12.3 数据库配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| database.url | postgres://... | 连接字符串 |
| database.max_connections | 10 | 最大连接数 |

### 12.4 JWT 配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| jwt.secret | (必填) | 签名密钥 |
| jwt.access_token_ttl | 86400 | Access Token 有效期 (秒) |
| jwt.refresh_token_ttl | 604800 | Refresh Token 有效期 (秒) |
| jwt.auto_refresh_threshold | 3600 | 自动刷新阈值 (秒) |

### 12.5 邮件配置

| 配置项 | 说明 |
|--------|------|
| email.smtp_host | SMTP 服务器地址 |
| email.smtp_port | SMTP 端口 |
| email.smtp_user | SMTP 用户名 |
| email.smtp_pass | SMTP 密码 |
| email.from_email | 发件人邮箱 |
| email.from_name | 发件人名称 |

---

## 13. 启动流程

```
伪代码:

fn main():
    // 1. 初始化
    init_logging()
    config = load_config()
    
    // 2. 数据库
    db_pool = create_connection_pool(config.database)
    run_migrations(db_pool)
    
    // 3. 创建服务实例
    jwt_validator = JwtValidator::new(config.jwt)
    email_service = EmailService::new(config.email)
    user_service = UserService::new(db_pool)
    token_service = TokenService::new(db_pool, config.jwt)
    
    // 4. 启动 Axum Auth API (后台任务)
    spawn_async:
        app = create_axum_router(services)
        listen_on("127.0.0.1:{api_port}")
    
    // 5. 启动 Pingora Gateway (主线程)
    gateway = AuthGateway::new(jwt_validator, upstreams)
    server = PingoraServer::new()
    server.add_http_proxy(gateway, "0.0.0.0:{gateway_port}")
    server.run_forever()
```

---

## 14. arc-generater 集成

### 14.1 集成方式

arc-generater 无需实现认证逻辑，只需读取 Gateway 注入的请求头:

- `X-User-Id`: 当前用户 ID (已通过 JWT 验证)

### 14.2 修改点

1. **添加依赖注入函数:** 从 Header 提取 `X-User-Id`，无则返回 401
2. **Session 模型扩展:** 添加 `user_id` 字段关联用户

---

## 15. 实现优先级

| Phase | 模块 | 说明 |
|-------|------|------|
| 1 | 项目骨架 | Cargo.toml, 目录结构, 配置加载 |
| 1 | 数据库 | PostgreSQL 连接池, 迁移脚本 |
| 2 | Axum Auth API | 注册/登录/刷新端点 |
| 2 | 邮件服务 | Stalwart 集成 |
| 3 | Pingora Gateway | JWT 验证, 路由转发 |
| 3 | Token 自动刷新 | X-New-Access-Token 机制 |
| 4 | 速率限制 | IP/邮箱限流 |
| 4 | arc-generater 集成 | X-User-Id 读取 |

---

## 16. 依赖清单

### 16.1 核心依赖

| 类别 | 依赖 | 用途 |
|------|------|------|
| Gateway | pingora, pingora-core, pingora-proxy | HTTP 代理 |
| Web 框架 | axum, tower, tower-http | Auth API |
| 异步运行时 | tokio | 异步 I/O |
| 数据库 | sqlx (postgres, uuid, chrono) | PostgreSQL 访问 |
| 认证 | jsonwebtoken, argon2 | JWT + 密码哈希 |
| 邮件 | mail-builder, mail-send | SMTP 发送 |
| 序列化 | serde, serde_json | JSON 处理 |
| 配置 | config, dotenvy | 配置管理 |
| 日志 | tracing, tracing-subscriber | 结构化日志 |
| 错误 | thiserror, anyhow | 错误处理 |
| 时间 | chrono | 时间处理 |
| 其他 | uuid, async-trait | 工具库 |

### 16.2 注意事项

- 移除 `deadpool-postgres` (与 sqlx 功能重复)
- Pingora 主要支持 Linux，Windows 开发需使用 WSL

---

## 17. 测试计划

### 17.1 单元测试

| 测试项 | 验证内容 |
|--------|----------|
| 密码验证 | 强密码通过，弱密码拒绝 |
| 邮箱验证 | 合法格式通过，非法格式拒绝 |
| JWT 往返 | 生成 → 验证 → 解析 Claims 一致 |
| 密码哈希 | 哈希 → 验证匹配 |

### 17.2 集成测试

| 测试项 | 验证内容 |
|--------|----------|
| 注册流程 | 发送验证码 → 验证 → 创建用户 |
| 登录流程 | 正确凭证登录成功，错误凭证拒绝 |
| Token 刷新 | 有效 Refresh Token 获取新 Access Token |
| 速率限制 | 超限请求返回 429 |

---

## 18. 邮件模板

### 18.1 验证码邮件

**主题:** ARC 验证码

**内容结构:**
- 标题: "您的验证码"
- 验证码: 大号加粗蓝色字体，字符间距加大
- 提示: "有效期 10 分钟，请勿泄露给他人"
- 页脚: "如果您没有请求此验证码，请忽略此邮件"

### 18.2 密码重置邮件

**主题:** 重置密码

**内容结构:**
- 标题: "重置密码"
- 说明: "您正在重置 ARC 账户密码"
- 验证码: 大号加粗红色字体
- 警告: "有效期 10 分钟。如非本人操作，请立即修改密码"
