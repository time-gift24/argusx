# Cookie Capture 设计文档

## 概述

**目的**: 为 ArgusX Desktop 实现自动化的 Cookie 捕获和代理功能，支持公司内部工具的 SSO 讁证。

**核心能力**:
- 自动捕获 Chrome 中已认证用户的 cookies
- 通过白名单限制 cookie 捕获范围
- 描述用户 opt-in 控制管理隐私
- 为 Agent 提供统一的 cookie 共享池(按域名)
- HTTP 代理功能，自动为请求附加对应 cookies

## 架构

```
┌─────────────────────────────────────────────────────────┐
│                    Desktop App (Tauri)                  │
│                                                         │
│  ┌─────────────┐         ┌──────────────────────┐    │
│  │  UI Layer   │         │  cookie-gateway       │    │
│  │  (Next.js)  │         │  (独立 crate)          │    │
│  └─────────────┘         │  ┌────────────────┐  │    │
│                          │  │ Cookie Store    │  │    │
│                          │  │  (Memory)           │  │    │
│                          │  └────────────────┘  │    │
│                          │  ┌────────────────┐  │    │
│                          │  │ HTTP Proxy      │  │    │
│                          │  └────────────────┘  │    │
└─────────────────────────────────────────────────────────┘
         ▲                                    ▲
         │ POST cookies                        │ Proxy requests
         │                                    │
    ┌────────────────┐              ┌──────────────────┐
    │ Chrome Extension│              │   Agent Runtime  │
    │ (新建)           │              │   (已存在)        │
    └────────────────┘              └──────────────────┘
```

## 核心组件

### 1. cookie-gateway (新建 Rust Crate)

**位置**: `cookie-gateway/`

**依赖**:
```toml
[dependencies]
axum = { version = "0.8.8", features = ["json"] }
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true
anyhow.workspace = true
thiserror.workspace = true
```

**核心模块**:
- `store.rs`: CookieStore (内存存储 + 白名单验证)
- `gateway.rs`: HTTP Server (axum)
- `proxy.rs`: HTTP Proxy handler
- `config.rs`: 配置(硬编码白名单)
- `lib.rs`: 公共 API

**HTTP Endpoints**:
```
POST /api/cookies              # Extension 上传 cookies
  Body: { "domain": "api.company.com", "cookies": [...] }

GET  /api/cookies?domain=xxx   # Agent 获取 cookies
  Response: { "cookies": [...] }

POST /api/proxy                # Agent 发起代理请求
  Body: { "url": "...", "method": "GET", "headers": {} }
  Response: { "status": 200, "body": "...", "headers": {} }

GET  /api/health                  # 健康检查
  Response: { "status": "ok" }

GET  /api/whitelist             # 获取白名单
  Response: { "whitelist": [...] }
```

