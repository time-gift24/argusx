# Runtime 启动与真实前端接入设计

日期: 2026-03-08

## 目标

为 ArgusX 引入一个与启动方式无关的 `runtime` 层，统一负责：

- 读取并自动创建 `~/.argusx/argusx.toml`
- 解析 SQLite 路径、日志文件路径、telemetry 配置
- 初始化本地日志
- 以“失败可降级”的方式初始化 ClickHouse telemetry
- 初始化 SQLite 和 session 持久化
- 构造并初始化 `SessionManager`
- 为 desktop 以及后续其他启动方式提供统一运行时入口

同时满足以下约束：

- `SessionManager` 不放在 `desktop` 中
- `sqlite.db` 默认位于 `~/.argusx/sqlite.db`
- telemetry、日志文件路径、sqlite 路径都统一由 `~/.argusx/argusx.toml` 配置
- `argusx.toml` 不存在时自动创建默认文件
- telemetry 开启但 ClickHouse 不可达时，应用继续启动，只在日志中记录，不向前端暴露 degraded 状态

## 非目标

- 本文档不包含具体 implementation task 拆分
- 不在 desktop 中实现真实 `ModelRunner / ToolRunner / ToolAuthorizer / TurnObserver`
- 不做配置热加载
- 不实现 telemetry 本地补发队列
- 不引入“前端专属状态存储”，前端仍通过现有 command/event bridge 接收 session 运行时状态

## 背景问题

当前 desktop 启动存在以下问题：

- telemetry 配置读取硬编码为 `config/telemetry.toml`
- SQLite 连接硬编码为 `sqlite:argusx.db`
- 启动逻辑分散在 desktop 中，不利于后续 CLI、daemon 或测试入口复用
- telemetry 与主路径缺少清晰的失败边界
- 配置、路径、资源初始化顺序没有统一抽象

这些问题在接入真实前端和真实 ClickHouse 后会变成启动语义不稳定、环境依赖泄漏和后续复用困难。

## 设计原则

### 1. runtime 是资源 owner，desktop 是接入层

`runtime` 负责配置、资源与运行时服务的创建和销毁。

`desktop` 只负责：

- 调用 `runtime` 提供的构造入口
- 将运行时句柄接到 Tauri state
- 提供 commands
- 进行事件桥接

这保证 `SessionManager`、SQLite、telemetry 不被绑定到某个 UI 壳中。

### 2. SQLite/session 是主路径，telemetry 是旁路

应用是否可用由主路径决定：

- 配置可读取
- SQLite 可初始化
- `SessionManager` 可创建

telemetry 失败不应阻断应用启动。它只影响可观测性，不影响对话与持久化功能。

### 3. 配置只有一个入口

用户侧只维护一个文件：

- `~/.argusx/argusx.toml`

不再把 telemetry、日志、SQLite 分散到多个路径或多个配置源中。

### 4. 显式路径优于隐式推断

配置文件中显式保存：

- SQLite 文件路径
- 日志文件路径

默认值可以自动生成，但一旦配置落盘，后续启动应以配置为准，而不是再在代码里偷偷拼路径。

### 5. 日志先于 telemetry

为了保证 telemetry 初始化失败也有记录，必须先初始化本地日志，再尝试启动 telemetry。

## 备选方案

### 方案 A：继续把启动逻辑留在 desktop

做法：

- desktop 直接读 `~/.argusx/argusx.toml`
- desktop 直接初始化 SQLite、telemetry、`SessionManager`

优点：

- 改动最少
- 落地最快

缺点：

- `desktop` 事实成为 runtime owner
- 后续 CLI 或其他启动方式无法复用
- Tauri 壳与核心资源生命周期耦合过深

### 方案 B：新增独立 `runtime` crate

做法：

- 新增 `runtime` crate，统一提供配置加载与运行时构造
- desktop、未来 CLI、测试入口都依赖它

优点：

- 边界清晰
- 复用性高
- 最符合当前约束

缺点：

- 需要新增一层抽象
- 初始改动比方案 A 大

### 方案 C：完整 runtime service 化

做法：

- 在方案 B 基础上继续引入配置服务、状态服务、热重载和资源管理器

优点：

- 长期演进空间最大

缺点：

- 当前阶段过重
- 会稀释真实接入的核心目标

**结论：采用方案 B。**

## crate 边界

### `session`

保留职责：

- `ThreadStore`
- `SessionManager`
- thread/turn 持久化模型
- thread/turn 编排逻辑

明确不负责：

- 读取 `~/.argusx/argusx.toml`
- 选择日志路径
- 初始化 telemetry
- 感知 Tauri

### `runtime`

新增职责：

- 定位并确保 `~/.argusx/`
- 自动创建默认 `argusx.toml`
- 读取和校验配置
- 解析绝对路径
- 初始化本地日志
- 尝试初始化 telemetry
- 初始化 SQLite 与 session schema
- 创建并初始化 `SessionManager`
- 暴露统一 shutdown 行为

### `desktop`

保留职责：

- 调用 `runtime::build_runtime()`
- 将 `SessionManager` 句柄注入 Tauri state
- command 参数解析
- `SessionEvent -> 前端 payload` 事件桥接
- 进程级生命周期挂接

明确不负责：

- 持有 `SessionManager` 的创建逻辑
- 决定 telemetry 失败策略
- 决定 SQLite 默认路径
- 决定配置文件位置

## 配置模型

建议在 `runtime` 中定义顶层配置，而不是直接把 `telemetry::TelemetryConfig` 当成应用总配置。

```rust
pub struct AppConfig {
    pub paths: PathsConfig,
    pub telemetry: TelemetrySection,
}

pub struct PathsConfig {
    pub sqlite: std::path::PathBuf,
    pub log_file: std::path::PathBuf,
}

pub struct TelemetrySection {
    pub enabled: bool,
    pub clickhouse_url: String,
    pub database: String,
    pub table: String,
    pub high_priority_batch_size: usize,
    pub low_priority_batch_size: usize,
    pub high_priority_flush_interval_ms: u64,
    pub low_priority_flush_interval_ms: u64,
    pub max_in_memory_events: usize,
    pub max_retry_backoff_ms: u64,
    pub full_logging: bool,
    pub delta_events: bool,
}
```

默认配置文件内容建议为：

```toml
[paths]
sqlite = "~/.argusx/sqlite.db"
log_file = "~/.argusx/argusx.log"

[telemetry]
enabled = true
clickhouse_url = "http://localhost:8123"
database = "argusx"
table = "telemetry_logs"
high_priority_batch_size = 5
low_priority_batch_size = 500
high_priority_flush_interval_ms = 1000
low_priority_flush_interval_ms = 30000
max_in_memory_events = 10000
max_retry_backoff_ms = 30000
full_logging = false
delta_events = false
```

### 配置解析规则

- `~` 必须在 runtime 中展开为绝对路径
- 相对路径如果被允许，必须相对于 `~/.argusx/` 解析
- `sqlite` 和 `log_file` 的父目录必须在启动时自动确保存在
- 配置文件不存在时自动写入默认内容
- 配置字段缺失或非法时，启动失败并输出明确错误

## runtime API 设计

建议对外提供以下边界：

```rust
pub struct ArgusxRuntime {
    pub config: std::sync::Arc<AppConfig>,
    pub sqlite_pool: sqlx::SqlitePool,
    pub session_manager: session::manager::SessionManager,
    pub telemetry: Option<telemetry::TelemetryRuntime>,
}

pub fn ensure_app_config() -> anyhow::Result<(std::path::PathBuf, AppConfig)>;

pub async fn build_runtime() -> anyhow::Result<ArgusxRuntime>;
```

其中：

- `ensure_app_config()` 负责目录、默认文件、路径展开、最小校验
- `build_runtime()` 负责实际资源初始化

建议再提供：

```rust
impl ArgusxRuntime {
    pub fn telemetry_enabled(&self) -> bool;
    pub fn session_manager(&self) -> &session::manager::SessionManager;
    pub fn into_parts(self) -> ArgusxRuntimeParts;
    pub fn shutdown(self, timeout: std::time::Duration) -> anyhow::Result<()>;
}
```

其中 `shutdown()` 的重点是：

- 如果 telemetry runtime 存在，执行有界 shutdown
- SQLite pool 和 `SessionManager` 依赖 drop 收尾

## 启动顺序

推荐固定为以下顺序：

1. 定位用户 home 目录
2. 确保 `~/.argusx/` 存在
3. 确保 `~/.argusx/argusx.toml` 存在，不存在则写默认文件
4. 读取并解析配置
5. 展开 `sqlite` / `log_file` 路径，确保父目录存在
6. 初始化本地日志
7. 尝试初始化 telemetry
8. telemetry 初始化失败时记录 warning，并以 `None` 继续
9. 连接 SQLite
10. 初始化 session schema
11. 创建 `ThreadStore`
12. 创建 `SessionManager`
13. 调用 `SessionManager::initialize()`
14. 返回 `ArgusxRuntime`

这样设计的原因：

- 先有日志，再有 telemetry 失败记录
- 先有配置，再有资源创建
- 先有 SQLite/session 主路径，再进入 UI 或其他接入层

## telemetry 降级语义

本次明确采用以下策略：

- 配置中 telemetry 开启时，runtime 会尝试初始化 telemetry
- 如果 ClickHouse 不可达、writer 初始化失败或 telemetry runtime 启动失败：
  - 记录一条本地 warning/error 日志
  - 返回 `telemetry: None`
  - 整个应用继续启动

前端语义：

- 不向前端暴露 degraded 状态
- 前端不新增 telemetry 健康状态 API

这保证 telemetry 只影响观测，不改变用户会话可用性。

## 本地日志设计

日志文件路径由 `paths.log_file` 配置。

第一版只要求：

- 进程启动时能写入单文件
- telemetry 初始化失败时有明确日志
- runtime 构建失败时有明确日志

第一版不要求：

- 日志轮转
- 多文件分级
- 前端可查看日志

如果后续需要轮转，可以在 `runtime` 内继续扩展，不影响 `session` 和 `desktop` 边界。

## SQLite 设计

SQLite 文件路径由 `paths.sqlite` 配置，默认值为：

- `~/.argusx/sqlite.db`

runtime 负责：

- 把路径转换成 `sqlx` 可接受的 SQLite DSN
- 确保父目录存在
- 连接数据库
- 初始化 session schema

`session` 不负责路径推导，只接受现成的 `SqlitePool`。

## desktop 接入方式

desktop 启动时只做：

```rust
let runtime = runtime::build_runtime().await?;
let session_manager = runtime.session_manager.clone();
```

然后：

- 将 `SessionManager` 或其包装句柄注入 Tauri state
- 启动 session event bridge
- 注册 commands
- 在应用退出时调用 `runtime.shutdown(timeout)`

desktop 中不再出现：

- `config/telemetry.toml`
- `sqlite:argusx.db`
- `SessionManager::new(...)`
- telemetry 失败策略判断

这些逻辑都必须收敛到 `runtime` 中。

## 对未来其他启动方式的影响

采用 `runtime` 层后，未来可以自然扩展：

- CLI 启动方式
- headless daemon
- 集成测试入口
- 单独的 maintenance/migration 工具

它们都共享：

- 配置解析规则
- 默认目录与默认配置
- telemetry 降级语义
- SQLite 初始化与 `SessionManager` 初始化流程

## 错误处理策略

### 启动必须失败的错误

- 无法定位 home 目录
- 无法创建 `~/.argusx/`
- 无法创建默认配置文件
- 配置文件 TOML 非法或关键字段非法
- 无法创建日志文件
- SQLite 无法连接
- session schema 初始化失败
- `SessionManager::initialize()` 失败

### 启动可降级的错误

- telemetry 配置开启但 ClickHouse 不可达
- telemetry writer 初始化失败
- telemetry runtime 线程启动失败

### 记录要求

- 所有降级都必须至少写入本地日志
- 错误日志必须包含失败阶段和原始错误

## 测试与验证

设计上至少需要覆盖以下验证面：

### 配置与路径

- 不存在 `~/.argusx/argusx.toml` 时自动创建默认配置
- `~` 路径可正确展开
- 相对路径解析行为稳定

### telemetry 降级

- ClickHouse 不可达时 `build_runtime()` 仍成功
- telemetry 降级时 `telemetry == None`
- 本地日志中有对应 warning/error

### SQLite 主路径

- 默认 SQLite 路径可创建并连接
- schema 初始化后 `SessionManager` 可正常列线程/创建线程

### desktop 接入

- desktop 通过 `runtime` 成功获取 `SessionManager`
- desktop 不再依赖硬编码配置路径和硬编码 SQLite DSN

## 迁移原则

实施时应遵循：

1. 先新增 `runtime` crate，不立刻删除 desktop 旧逻辑
2. 将配置和路径逻辑先迁入 `runtime`
3. 再把 telemetry 初始化迁入 `runtime`
4. 再把 SQLite + `SessionManager` 初始化迁入 `runtime`
5. 最后让 desktop 只消费 `runtime`

这样可以降低一次性迁移的启动回归风险。

## 结论

本次采用独立 `runtime` crate 作为统一运行时组装层：

- `session` 继续管理会话与 thread/turn 编排
- `runtime` 负责配置、日志、telemetry、SQLite、`SessionManager` 的真实初始化
- `desktop` 只做 Tauri 壳、command 与事件桥接

该方案满足当前真实前端接入需求，也为后续非 desktop 启动方式保留稳定复用边界。
