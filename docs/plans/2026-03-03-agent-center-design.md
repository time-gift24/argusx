# Agent-Center 设计文档

**版本:** 1.0
**日期:** 2026-03-03
**作者:** Claude + ArgusX Team

## 执行摘要

基于 opencode 和 codex 的 subagent 宔现经验，设计一个新的 `agent-center` crate，为系统提供 **多 agent 协作能力**。

**核心特性:**
- Agent 注册与管理：从 `.agents/*.toml` 加载定义，存入 SQLite
- Thread 管理：轻量级并发抽象，共享 Session 但有独立的 context
- 权限控制：多层权限系统（深度限制、工具白名单、继承规则)
- 内置工具：spawn_agent, wait, close_agent
- 可靠性保障：RAII Guard、深度限制、并发控制
- 错误恢复：重试机制
- 持久化：SQLite 存储
- 配置热重载：支持运行时更新

**设计原则:**
- **最小侵入**：在现有 agent-core/agent-session/agent-tool 基础上做最小改动
- **向后兼容**：现有代码继续工作(通过可选字段和默认值)
- **渐进增强**：先实现核心功能，后续逐步增强
- **职责分离**：Session 管理 session，Thread 管理并发，权限控制隔离

## 架构层次
```
┌── Desktop UI (Tauri + Next.js)
│   └── agent (Facade)
│       └── agent-center (Thread + Registry) ← NEW!
│           └── agent-session (SessionRuntime)
│               └── agent-tool (ToolRegistry)
│                   └── agent-core (Core traits)
```

## 模块结构
```
agent-center/
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── core/
│   │   ├── registry.rs          # Agent 定义注册表
│   │   ├── thread_pool.rs       # Thread 并发池
│   │   ├── lifecycle.rs         # Thread 生命周期
│   │   └── executor.rs         # 执行逻辑
│   ├── permission/
│   │   ├── mod.rs
│   │   ├── context.rs          # PermissionContext
│   │   ├── ruleset.rs          # 权限规则
│   │   ├── guard.rs            # RAII Guards
│   │   └── evaluator.rs       # 权限评估
│   ├── persistence/
│   │   ├── mod.rs
│   │   ├── store.rs            # AgentDefinitionStore
│   │   └── thread_state.rs    # ThreadStateStore
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── spawn.rs            # spawn_agent 工具
│   │   ├── wait.rs             # wait 工具
│   │   └── close.rs           # close_agent 工具
│   ├── config/
│   │   ├── mod.rs
│   │   ├── loader.rs            # ConfigLoader
│   │   ├── watcher.rs          # HotReloader (可选)
│   │   └── types.rs             # AgentDefinition
│   └── api/
        └── center.rs          # AgentCenter facade
```

## 核心数据结构
```rust
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub version: String,
    pub prompt: AgentPrompt,
    pub tools: ToolsConfig,
    pub permissions: PermissionConfig,
    pub limits: LimitsConfig,
}

pub struct Thread {
    pub thread_id: ThreadId,
    pub session_id: SessionId,        // 独立的 session
    pub parent_thread_id: Option<ThreadId>,
    pub parent_session_id: Option<SessionId>,
    pub agent_name: String,
    pub status: ThreadStatus,
    pub depth: u32,
    pub permission_context: PermissionContext,
    pub created_at: i64,
    pub ended_at: Option<i64>,
}
```

## 关键设计决策
1. **Thread 隔离模型**：每个 thread 创建独立 session（选项 B: 更清晰的数据隔离)
2. **权限系统**：在 agent-center 层，不在 core 层
3. **最小改动原则**：复用现有 SessionRuntime
4. **并发控制**：使用 ThreadPool 而不是修改 SessionRuntime
5. **深度限制**：SpawnGuard RAII 机制
6. **配置热重载**：监听文件变化，运行时更新
7. **持久化**：SQLite 存储
8. **工具注册**：注册到现有 agent-tool
9. **错误恢复**：重试机制和错误传播
10. **测试覆盖**：单元测试、集成测试、错误测试

## 示例配置
```toml
[agent]
Name = "explorer"
Description = "Fast agent specialized for exploring codebases"
Prompt = """
You are a fast agent specialized for exploring codebases.
Use glob, grep, and read tools to quickly find information.
Always be thorough, balancing depth and workload.
- "quick": Basic searches
- "medium": Moderate exploration
- "very thorough": Comprehensive analysis
"""
tools = ["read", "grep", "glob", "ls"]
permissions = {inherit = true}
limits = {max_depth = 2, max_concurrent = 3
```
## 集成方式
```rust
// 在 desktop 初始化
let center = AgentCenter::builder()
    .model_provider(model_provider)
    .tool_registry(tool_registry)
    .store_path(".agent/center.db")
    .agents_dir(".agents")
    .build()
    .await?;

// 注册内置工具
agent_center.register_builtin_tools(&tool_registry).await?;
```
## 数据库 Schema
```sql
-- agent_definitions table
CREATE TABLE agent_definitions (
    name TEXT PRIMARY KEY,
    description TEXT,
    version TEXT,
    prompt TEXT NOT NULL,
    tools TEXT, -- JSON
    permissions TEXT, -- JSON
    limits TEXT, -- JSON
    created_at INTEGER,
    updated_at INTEGER
);

-- threads table
CREATE TABLE threads (
    thread_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    parent_thread_id TEXT,
    parent_session_id TEXT,
    agent_name TEXT NOT NULL,
    depth INTEGER NOT NULL,
    status TEXT NOT NULL,
    created_at INTEGER,
    ended_at INTEGER
);

-- Indexes
CREATE INDEX idx_agent_name ON threads(agent_name);
CREATE INDEX idx_parent_session_id ON threads(parent_session_id);
CREATE INDEX idx_session_id ON threads(session_id);
CREATE INDEX idx_status ON threads(status);
```
## 宽度规划
**Phase 1: 核心框架 (3-4 天)
- AgentRegistry + ConfigLoader
- 基础的 Thread 创建和执行
- spawn_agent 工具

- SQLite 持久化
- 错误处理
**Phase 2: 可靠性机制 (2-3 天)
- 权限控制和 Guard
- 深度和并发限制
- wait/close 工具
**Phase 3** 增强功能 (可选)
- 热重载配置
- API 更新提示词
- 监控和日志
## 后续增强
- API 更新 agent 定义
- Agent 统计和监控
- 高级通知机制
- 更多内置工具
- 配置验证和优化
- 性能优化
