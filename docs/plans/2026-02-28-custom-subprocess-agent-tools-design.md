# 高性能定制化 Agent 子进程工具设计

## 背景与目标

当前工具体系在 `agent_core::tools::{ToolCatalog, ToolExecutor}` 抽象下运行良好，但内建实现主要面向仓库内工具（如 `read`/`shell`）。

本设计目标是支持**用户自行实现的编码级工具**，同时满足：

1. 高性能（低序列化开销、低首调延迟、稳定并发）
2. 高开发速度（用户最少样板代码）
3. 高定制化（不对齐 MCP，不受 MCP 语义约束）

## 已确认决策

1. 运行隔离：子进程插件（非同进程）
2. 生命周期：常驻进程，按 `tool_name` 复用
3. 注册方式：配置文件显式声明
4. 通信协议：非 MCP，自定义协议
5. 编码格式：Length-Prefixed + MessagePack
6. 启动策略：混合模式（热工具预热，冷工具懒启动）

## 非目标

1. 不实现 MCP 兼容层
2. 不引入 HTTP/gRPC 传输
3. 不做多机分布式工具调度
4. 不在 v1 支持动态在线安装/卸载工具

## 架构设计

### 分层保持

保持上层执行链路不变：

- `agent-turn` 仍通过 `ToolCatalog + ToolExecutor` 调用工具
- `agent-session` 无需感知进程管理

新增能力集中在 `agent-tool`：

- `SubprocessToolRuntime`：实现 `ToolCatalog + ToolExecutor`
- `ToolHostManager`：管理子进程生命周期
- `ProcessSupervisor`：负责保活/重启/熔断
- `ToolConfigLoader`：加载并校验工具配置

### 组件职责

1. `ToolConfigLoader`
- 读取 `tools.toml`
- 校验 `tool_name` 唯一性、可执行文件存在、schema 合法、限流配置合法
- 输出 `ToolDefinition`

2. `SubprocessToolRuntime`
- `list_tools()` / `tool_spec()`：从 `ToolDefinition` 直接映射
- `execute_tool()`：转发到 `ToolHostManager` 并映射结果

3. `ToolHostManager`
- `HashMap<tool_name, ToolHostHandle>`
- 懒启动与预热并存
- 每工具队列与并发隔离，防止相互拖垮

4. `ToolHostHandle`
- 维护单个子进程与 stdio 通道
- 请求路由表：`call_id -> oneshot sender`
- 单读循环 + 写队列模型

5. `ProcessSupervisor`
- 握手检测、崩溃检测、退避重启、熔断/半开恢复

## 协议设计（自定义，非 MCP）

### 传输帧

- 帧格式：`[u32_le payload_len][msgpack payload]`
- 方向：双向全双工（stdio）
- 要求：每帧完整，不跨帧拼 JSON 文本

### 消息类型

1. `InitReq` / `InitResp`
- 建连后一次握手
- 返回工具版本、运行状态、可选元数据

2. `CallReq`
- 字段：`call_id`, `tool_name`, `arguments`, `context`

3. `CallResp`
- 成功：`call_id`, `output`, `is_error=false`
- 业务错误：`call_id`, `error_kind=User`, `message`
- 临时错误：`call_id`, `error_kind=Retryable`, `message`
- 致命错误：`call_id`, `error_kind=Fatal`, `message`

4. `ShutdownReq`
- 主进程优雅退出时发送

5. `CancelReq`（可选）
- 超时场景下通知子进程取消指定 `call_id`

### 协议约束

1. 响应必须回传同一 `call_id`
2. 未知消息类型视为协议错误（`Fatal`）
3. 解码失败计入协议错误计数并触发熔断评估

## 执行与并发模型

1. 每个 `tool_name` 默认 1 个常驻进程（v1）
2. 每工具独立请求队列（`max_queue`）
3. `EffectExecutor` 的并发控制与工具队列并行生效
4. 队列满时快速失败为 `Retryable`（避免全局背压）
5. 工具调用超时后当前调用失败；若进程失活则重启

## 生命周期与启动策略

### 混合预热

- `warmup=true`：启动时预热（并发预热上限可配置，如 4）
- `warmup=false`：首次调用时懒启动
- 预热失败不阻断 agent 启动，标记 unhealthy

### 重启与熔断

1. 崩溃后按指数退避重启（`restart_backoff_ms`）
2. 连续失败超过阈值进入 `Open` 熔断状态
3. 冷却后进入 `HalfOpen`，单请求探测恢复
4. 探测成功回到 `Closed`

### 关闭流程

1. 停止接收新请求
2. 等待 in-flight 完成（上限超时）
3. 发送 `ShutdownReq`
4. 超时未退出则强杀

## 错误语义映射

统一映射到 `ToolExecutionErrorKind`：

1. `User`：参数/业务错误，不重试
2. `Runtime`：子进程非致命运行错误
3. `Transient`：超时、队列满、短暂不可用（可重试）
4. `Internal`：协议损坏、路由表破坏、不可恢复状态

## 配置模型草案

`tools.toml` 示例：

```toml
[[tools]]
name = "my_read"
command = "/usr/local/bin/my-read-tool"
args = ["--mode", "fast"]
warmup = true
max_queue = 128
call_timeout_ms = 2000
restart_backoff_ms = 500

[tools.env]
MY_TOOL_LEVEL = "info"

[tools.execution]
parallel_mode = "parallel_safe"
max_retries = 2
backoff_ms = 200

[tools.schema]
type = "object"
properties.path.type = "string"
required = ["path"]
```

## 开发体验设计（SDK）

提供 `agent_tool_sdk`（Rust crate）：

1. 用户只需实现业务 trait：`handle(args, ctx) -> Result<Value, ToolError>`
2. SDK 负责 MessagePack 编解码、帧收发、错误封装
3. SDK 提供 `main_loop()` 模板，减少重复样板代码

## 测试与验收标准

### 测试层次

1. 单元测试
- 帧编解码
- `call_id` 路由
- 超时与取消
- 熔断状态机

2. 集成测试
- 假工具进程：正常/慢响应/崩溃/乱序响应/坏包
- 多并发下队列与背压行为

3. 性能回归
- 高并发场景吞吐不显著退化
- P95/P99 延迟受控

### DoD

1. 100 并发调用下无死锁/无资源泄漏
2. 子进程异常后可自动恢复
3. 错误分类稳定可预测
4. agent 关闭流程可在超时上限内完成

## 风险与缓解

1. 风险：用户工具实现质量参差
- 缓解：SDK 默认超时、panic 保护、结构化错误规范

2. 风险：长尾慢调用导致队列堆积
- 缓解：按工具队列隔离 + 快速失败 + 熔断

3. 风险：协议演进破坏兼容
- 缓解：协议版本字段 + 向后兼容策略

## 里程碑建议

1. M1：最小可用子进程 runtime（懒启动 + call + timeout）
2. M2：混合预热、重启、熔断、可观测性
3. M3：SDK 与文档模板，开放用户自定义工具接入
