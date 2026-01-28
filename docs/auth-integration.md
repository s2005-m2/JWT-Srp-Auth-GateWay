# 接入登录注册指南

本文档介绍如何在前端应用中接入 ARC Auth 的 SRP (Secure Remote Password) 认证系统。

## 概述

ARC Auth 使用 SRP-6a 协议进行用户认证，这是一种零知识密码证明协议：
- 服务端永远不会接收到用户的明文密码
- 即使数据库泄露，攻击者也无法反推出密码
- 双向认证，防止钓鱼攻击

## 前置依赖

### JavaScript/TypeScript

```bash
npm install secure-remote-password
# 或
pnpm add secure-remote-password
```

## 注册流程

### 步骤 1: 请求验证码

```typescript
const response = await fetch('http://localhost:8080/auth/register', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ email: 'user@example.com' }),
});
```

### 步骤 2: 生成 SRP 凭证并完成注册

```typescript
import * as srp from 'secure-remote-password/client';

function generateRegistrationData(email: string, password: string) {
  const salt = srp.generateSalt();
  const privateKey = srp.derivePrivateKey(salt, email, password);
  const verifier = srp.deriveVerifier(privateKey);
  return { salt, verifier };
}

// 用户输入验证码后
const { salt, verifier } = generateRegistrationData(email, password);

const response = await fetch('http://localhost:8080/auth/register/verify', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'user@example.com',
    code: '123456',
    salt,
    verifier,
  }),
});

const data = await response.json();
// data: { user, access_token, refresh_token }
```

## 登录流程

SRP 登录是两步验证过程：

### 步骤 1: 初始化握手

```typescript
import * as srp from 'secure-remote-password/client';

const clientEphemeral = srp.generateEphemeral();

const initResponse = await fetch('http://localhost:8080/auth/login/init', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'user@example.com',
    client_public: clientEphemeral.public,
  }),
});

const initData = await initResponse.json();
// initData: { session_id, salt, server_public }
```

### 步骤 2: 计算证明并验证

```typescript
const privateKey = srp.derivePrivateKey(initData.salt, email, password);
const session = srp.deriveSession(
  clientEphemeral.secret,
  initData.server_public,
  initData.salt,
  email,
  privateKey
);

const verifyResponse = await fetch('http://localhost:8080/auth/login/verify', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    session_id: initData.session_id,
    client_proof: session.proof,
  }),
});

const loginData = await verifyResponse.json();
// loginData: { user, server_proof, access_token, refresh_token }
```

### 完整登录函数

```typescript
import * as srp from 'secure-remote-password/client';

async function login(email: string, password: string) {
  const clientEphemeral = srp.generateEphemeral();

  const initRes = await fetch('http://localhost:8080/auth/login/init', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, client_public: clientEphemeral.public }),
  });
  const initData = await initRes.json();
  if (!initRes.ok) throw new Error(initData.error?.message);

  const privateKey = srp.derivePrivateKey(initData.salt, email, password);
  const session = srp.deriveSession(
    clientEphemeral.secret,
    initData.server_public,
    initData.salt,
    email,
    privateKey
  );

  const verifyRes = await fetch('http://localhost:8080/auth/login/verify', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      session_id: initData.session_id,
      client_proof: session.proof,
    }),
  });
  return verifyRes.json();
}
```

## 密码重置

```typescript
// 步骤 1: 请求重置验证码
await fetch('http://localhost:8080/auth/password/reset', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ email: 'user@example.com' }),
});

// 步骤 2: 生成新凭证并重置
const { salt, verifier } = generateRegistrationData(email, newPassword);

await fetch('http://localhost:8080/auth/password/reset/confirm', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ email, code: '123456', salt, verifier }),
});
```

## Token 使用

### 访问受保护 API

```typescript
const response = await fetch('http://localhost:8080/api/your-endpoint', {
  headers: { 'Authorization': `Bearer ${access_token}` },
});
```

### 刷新 Token

```typescript
const response = await fetch('http://localhost:8080/auth/refresh', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ refresh_token }),
});
const data = await response.json();
// data: { access_token, refresh_token }
```

## 错误处理

| 错误码 | HTTP | 说明 |
|--------|------|------|
| `INVALID_EMAIL` | 400 | 邮箱格式无效 |
| `INVALID_CODE` | 400 | 验证码错误或过期 |
| `INVALID_CREDENTIALS` | 401 | 邮箱或密码错误 |
| `INVALID_TOKEN` | 401 | Token 无效 |
| `TOKEN_EXPIRED` | 401 | Token 过期 |
| `EMAIL_EXISTS` | 409 | 邮箱已注册 |
| `RATE_LIMITED` | 429 | 请求频率超限 |

## API 端点汇总

| 端点 | 方法 | 说明 |
|------|------|------|
| `/auth/register` | POST | 请求注册验证码 |
| `/auth/register/verify` | POST | 验证并创建账户 |
| `/auth/login/init` | POST | SRP 登录初始化 |
| `/auth/login/verify` | POST | SRP 登录验证 |
| `/auth/refresh` | POST | 刷新 Token |
| `/auth/password/reset` | POST | 请求密码重置 |
| `/auth/password/reset/confirm` | POST | 确认密码重置 |
