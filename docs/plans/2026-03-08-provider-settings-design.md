# Provider 配置与密钥存储设计

## 概述

本文档定义桌面应用的 provider 配置入口、SQLite 持久化模型和 API key 加密存储方案。

目标是让用户可以在右上角打开全局配置面板，维护多个 `OpenAI-compatible` provider profile，并指定一个全局默认 profile 供 `/chat` 页面发送消息时使用。

## 目标

- 在应用右上角提供统一的 provider 配置入口
- 将 provider profile 持久化到本地 SQLite
- 将 `api_key` 以密文形式存入 SQLite，而不是明文
- 支持多个 `OpenAI-compatible` profile
- 支持一个全局默认 profile，chat 运行时优先使用它
- 在没有 SQLite 配置时，继续支持现有环境变量回退路径

## 非目标

- 不在聊天页面内提供 provider 切换器
- 不支持用户自定义 provider 类型
- 不支持多默认 profile
- 不引入“主密码解锁”流程
- 不在 V1 中管理非 `OpenAI-compatible` provider profile

## 约束

- profile 主体数据保存在 SQLite
- `api_key` 必须以密文形式存储在 SQLite
- 用于加解密 `api_key` 的数据密钥不以明文形式落盘
- 数据密钥使用系统安全存储包装后保存
- 如果系统安全存储不可用，保存配置必须失败，不能退化为明文

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend (Next.js)                  │
│  ┌──────────────────────────────┐                          │
│  │ AppLayout Header             │                          │
│  │ - ProviderSettingsButton     │                          │
│  └──────────────┬───────────────┘                          │
│                 │ open dialog                               │
│  ┌──────────────▼───────────────┐                          │
│  │ ProviderSettingsDialog       │                          │
│  │ - profile list               │                          │
│  │ - create / edit / delete     │                          │
│  │ - set default                │                          │
│  │ - test connection            │                          │
│  └──────────────┬───────────────┘                          │
└─────────────────┼──────────────────────────────────────────┘
                  │ Tauri IPC
                  ▼
┌─────────────────────────────────────────────────────────────┐
│                      Backend (Rust Tauri)                  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Provider Settings Commands                          │  │
│  │ - list_provider_profiles                            │  │
│  │ - save_provider_profile                             │  │
│  │ - delete_provider_profile                           │  │
│  │ - set_default_provider_profile                      │  │
│  │ - test_provider_profile                             │  │
│  └───────────────────────┬──────────────────────────────┘  │
│                          │                                 │
│  ┌───────────────────────▼──────────────────────────────┐  │
│  │ ProviderProfileStore                                │  │
│  │ - SQLite persistence                                │  │
│  │ - unique default enforcement                        │  │
│  │ - list/load/save/delete                             │  │
│  └───────────────────────┬──────────────────────────────┘  │
│                          │                                 │
│  ┌───────────────────────▼──────────────────────────────┐  │
│  │ SecretVault                                         │  │
│  │ - create/load DEK                                   │  │
│  │ - wrap/unwrap via system secure storage             │  │
│  │ - encrypt/decrypt api_key                           │  │
│  └───────────────────────┬──────────────────────────────┘  │
│                          │                                 │
│  ┌───────────────────────▼──────────────────────────────┐  │
│  │ Chat Provider Resolver                              │  │
│  │ - load default profile from SQLite                  │  │
│  │ - fallback to env when no profile exists            │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## UI 设计

### 入口位置

配置按钮放在 [app-layout.tsx](/Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/components/layouts/app-layout.tsx) 顶部右侧，与主题切换并列。

### Dialog 行为

- 打开时加载全部 provider profile
- 列表项显示 `name`、`base_url`、`model`、`默认状态`
- 支持 `新增`、`编辑`、`设为默认`、`删除`
- 编辑已有 profile 时不回显 API key 原文
- 如果 API key 输入框留空，则表示“保持当前密文不变”
- 提供 `测试连接` 按钮，只做请求校验，不修改默认项

### 表单字段

- `名称`
- `Base URL`
- `Model`
- `API Key`

### 交互限制

- V1 仅允许 `OpenAI-compatible`
- 列表始终可以有多条 profile
- 同一时刻只能有一个默认 profile
- 默认 profile 不允许直接删除

## 数据模型

### SQLite 表

```sql
CREATE TABLE IF NOT EXISTS provider_profiles (
    id TEXT PRIMARY KEY,
    provider_kind TEXT NOT NULL,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    model TEXT NOT NULL,
    api_key_ciphertext BLOB NOT NULL,
    api_key_nonce BLOB NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_provider_profiles_single_default
ON provider_profiles(is_default)
WHERE is_default = 1;
```

### 字段语义

- `provider_kind`: 固定为 `openai_compatible`
- `api_key_ciphertext`: 使用应用数据密钥加密后的密文
- `api_key_nonce`: 对称加密所需随机 nonce
- `is_default`: 全局唯一默认项

## 密钥管理

### 方案

- 应用首次保存 profile 时生成一个随机数据密钥（DEK）
- DEK 不明文落 SQLite，也不写入普通配置文件
- DEK 使用系统安全存储包装后保存
- SQLite 仅保存 profile 数据和 `api_key` 密文

### 运行时流程

1. 首次保存 profile
2. 检查系统安全存储中是否已有包装后的 DEK
3. 如果没有，则生成新的 DEK，并写入系统安全存储
4. 使用 DEK 加密 API key
5. 将密文和 nonce 写入 SQLite

### 失败策略

- 如果系统安全存储不可用，保存配置直接失败
- 不允许退化到明文落盘
- 如果读取到密文但无法解密，视为配置损坏，保留数据并向 UI 返回错误

## 后端接口

建议新增以下 Tauri commands：

- `list_provider_profiles() -> ProviderProfileSummary[]`
- `save_provider_profile(input) -> ProviderProfileSummary`
- `delete_provider_profile(profile_id) -> ()`
- `set_default_provider_profile(profile_id) -> ProviderProfileSummary`
- `test_provider_profile(input) -> ProviderConnectionResult`

其中：

- `save_provider_profile` 同时支持创建和编辑
- 编辑时如果 `api_key` 为空，则复用旧密文
- `test_provider_profile` 使用临时输入直接发起最小请求，不要求先落库

## Chat 运行时集成

当前 [model.rs](/Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/src-tauri/src/chat/model.rs) 直接从环境变量读取 provider 配置。

V1 调整为：

1. 优先从 SQLite 加载默认 profile
2. 若存在默认 profile，则用其 `base_url`、`model`、`api_key`
3. 若 SQLite 中没有任何默认 profile，则回退到现有环境变量
4. 如果两者都不可用，则在启动 turn 时返回明确错误

## 错误处理

### 配置侧

- 表单校验失败：直接在 dialog 内联提示
- 默认项冲突：以后端唯一索引错误映射为用户可读消息
- 默认项删除：返回业务错误，不执行删除
- 系统安全存储失败：返回“无法初始化本机密钥存储”

### 运行时侧

- 无默认 profile 且无 env fallback：`start_turn` 返回配置错误
- 默认 profile 解密失败：`start_turn` 返回配置损坏错误
- 测试连接失败：仅提示失败原因，不修改已保存配置

## 测试策略

### Rust

- schema 初始化
- 创建 profile 后可以列出摘要
- 默认 profile 唯一性
- 更新 profile 且 `api_key` 为空时不覆盖密文
- 删除默认 profile 被拒绝
- SQLite 默认 profile 优先于环境变量
- 没有 SQLite 配置时 env fallback 仍然可用
- 密文写入数据库时不包含明文 key

### Frontend

- header 中显示 provider 配置按钮
- dialog 打开后加载 profile 列表
- 表单创建与编辑交互
- 默认项切换成功后 UI 刷新
- 编辑已有 profile 且不填写 API key 时不会清空
- 错误态提示可见

## 迁移与兼容性

- 该特性不要求迁移现有 env 配置到 SQLite
- 没有保存任何 profile 的用户保持当前行为
- 一旦设置了默认 profile，chat 默认走 SQLite 配置

## 结果

V1 完成后，用户可以在桌面应用右上角管理 `OpenAI-compatible` provider profile，API key 以密文落 SQLite，聊天运行时自动读取全局默认 profile，并在未配置 SQLite 时兼容现有环境变量流程。
