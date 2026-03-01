# Node.js + Rust CLI 重构设计

**日期**: 2026-03-01
**状态**: ✅ 已批准

## 概述

将现有的 Rust CLI (`agent-cli`) 重构为 Node.js 前端 + Rust 后端的架构，利用 Node.js 生态的优秀 CLI 库，提升开发体验和用户体验。

### 目标

- 利用 Node.js 生态的 CLI 库（commander, inquirer, ora 等）
- 保持 Rust 核心逻辑不变，最小化改动
- 提供更好的用户交互体验
- 支持单次执行和会话两种模式

### 方案选择

**选择方案 A：最小改动方案**

- Node.js CLI (`argusx`)：参数解析、配置管理、UI 交互
- Rust CLI (`argus-backend`)：保持现有的 agent-cli 逻辑，改用 JSON-RPC over stdio
- 工作量：2-3 天
- 风险：低，易于回退

## 架构设计

### 整体架构

```
┌─────────────────────────────────────┐
│      Node.js CLI (argusx)           │
│  - 参数解析 (commander)              │
│  - 配置管理                          │
│  - 用户交互 (确认、进度显示)          │
│  - 输出格式化                        │
└──────────────┬──────────────────────┘
               │ JSON-RPC over stdio
               ▼
┌─────────────────────────────────────┐
│     Rust Backend (argus-backend)     │
│  - Agent 核心逻辑                      │
│  - LLM 调用                           │
│  - 工具执行                           │
│  - 会话管理                           │
└─────────────────────────────────────┘
```

### 启动流程

```
用户执行: argusx agent "帮我重构这个文件"
    ↓
argusx.js 检测平台 → 定位 vendor/{target}/argus-backend
    ↓
spawn(argus-backend, ["--mode", "rpc-server"])
    ↓
建立 stdio 通道 (stdin/stdout)
    ↓
Node.js 发送 initialize 请求 (JSON-RPC)
    ↓
Rust 返回 initialized → 准备就绪
```

## 组件设计

### Node.js 侧 (`agent-js/`)

```
agent-js/
├── package.json
├── bin/
│   └── argusx.js          # 主入口，类似 codex.js
├── src/
│   ├── cli/               # 命令行参数解析
│   │   └── commands.ts
│   ├── rpc/               # JSON-RPC 客户端
│   │   ├── client.ts
│   │   └── types.ts
│   ├── ui/                # 用户界面组件
│   │   ├── prompts.ts     # 交互式确认
│   │   ├── progress.ts    # 进度显示
│   │   └── formatting.ts  # 输出格式化
│   └── config/            # 配置管理
│       └── loader.ts
└── vendor/                # 编译好的 Rust 二进制
    └── {target-triple}/
        └── argus-backend
```

**核心文件职责**：
- `argusx.js`：平台检测、二进制定位、spawn 启动
- `rpc/client.ts`：JSON-RPC 通信层
- `ui/prompts.ts`：使用 inquirer/ora 处理用户交互
- `cli/commands.ts`：使用 commander 解析参数

### Rust 侧 (`agent-cli/`)

保持现有结构，只修改：

```
agent-cli/src/
├── main.rs              # 入口：启动 JSON-RPC server
├── rpc/
│   ├── server.rs        # JSON-RPC server
│   ├── handlers.rs      # RPC 方法实现
│   └── protocol.rs      # 消息类型定义
├── agent/               # 现有 agent 逻辑（保持不变）
├── session/             # 现有会话管理（保持不变）
└── ...                  # 其他模块保持不变
```

## 数据流设计

### 1. Agent 执行流程

```
Node.js: call("agent.execute", {
  prompt: "帮我重构这个文件",
  session_id: null,
  tools: ["file", "exec"]
})
    ↓
Rust: 创建 Agent 会话
    ↓
Rust: LLM 推理 → 需要使用工具
    ↓
Rust: notification("tool.start", {
  tool: "file.read",
  params: { path: "src/main.rs" }
})
    ↓
Node.js: 显示 "📖 读取文件 src/main.rs..."
    ↓
Rust: notification("tool.end", {
  tool: "file.read",
  duration_ms: 123,
  success: true
})
    ↓
[重复 LLM → 工具调用循环]
    ↓
Rust: response("agent.execute", {
  success: true,
  output: "重构完成...",
  session_id: "xxx"
})
```

### 2. 流式输出

```
Rust: notification("content.delta", {
  content: "好的",
  reasoning: null
})
    ↓
Node.js: 实时显示 "好的"

Rust: notification("content.delta", {
  content: "，我来帮你",
  reasoning: "用户需要重构..."
})
    ↓
Node.js: 追加显示 "，我来帮你"
```

### 3. 工具权限确认

```
Rust: notification("tool.confirm", {
  id: 456,
  tool: "file.delete",
  params: { path: "important.txt" },
  risk: "high"
})
    ↓
Node.js: 弹出确认框
    ↓
用户: 点击 "确认"
    ↓
Node.js: call("tool.approve", { id: 456 })
    ↓
Rust: 执行工具 → 返回结果
```

## JSON-RPC 协议

### 请求格式

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "agent.execute",
  "params": {
    "prompt": "帮我重构这个文件",
    "session_id": null,
    "tools": ["file", "exec"],
    "options": {
      "model": "glm-5",
      "temperature": 0.7
    }
  }
}
```

### 响应格式

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "success": true,
    "output": "重构完成...",
    "session_id": "abc123",
    "tools_used": 3,
    "duration_ms": 5234
  }
}
```

### 通知格式

**流式输出：**
```json
{
  "jsonrpc": "2.0",
  "method": "content.delta",
  "params": {
    "content": "好的，我来帮你",
    "reasoning": "用户需求分析..."
  }
}
```

**工具确认：**
```json
{
  "jsonrpc": "2.0",
  "method": "tool.confirm",
  "params": {
    "id": 456,
    "tool": "file.delete",
    "params": { "path": "test.txt" },
    "risk": "high"
  }
}
```

### 核心 RPC 方法

| 方法 | 参数 | 返回值 |
|------|------|--------|
| `agent.execute` | `{prompt, session_id?, tools?, options?}` | `{output, session_id, ...}` |
| `agent.resume` | `{session_id}` | `{output, ...}` |
| `tool.approve` | `{id}` | `{success}` |
| `tool.reject` | `{id}` | `{success}` |
| `session.list` | `{}` | `{sessions: [...]}` |
| `session.delete` | `{session_id}` | `{success}` |
| `session.info` | `{session_id}` | `{info}` |
| `tool.list` | `{}` | `{tools: [...]}` |
| `tool.enable` | `{tool}` | `{success}` |
| `tool.disable` | `{tool}` | `{success}` |
| `history.get` | `{session_id}` | `{messages: [...]}` |

## 命令系统

### 命令分类

**本地命令（Node.js 处理）：**
- `/help`, `/?` - 显示帮助信息
- `/exit`, `/quit` - 退出程序
- `/clear` - 清屏
- `/config` - 查看/修改配置

**RPC 命令（转发到 Rust）：**
- `/sessions` - 列出所有会话
- `/session <id>` - 切换到指定会话
- `/new` - 创建新会话
- `/delete <id>` - 删除会话
- `/tools` - 列出可用工具
- `/enable <tool>` - 启用工具
- `/disable <tool>` - 禁用工具
- `/history` - 查看对话历史

### 命令帮助示例

```
可用命令：

会话管理:
  /sessions          列出所有会话
  /session <id>      切换到指定会话
  /new               创建新会话
  /delete <id>       删除会话

工具控制:
  /tools             列出可用工具
  /enable <tool>     启用工具
  /disable <tool>    禁用工具

配置:
  /config            查看配置
  /set <key> <val>   修改配置

其他:
  /help, /?          显示帮助
  /clear             清屏
  /exit, /quit       退出
```

## 错误处理

### 错误分类

**用户错误（友好提示）：**
- 参数错误：`argusx: error: missing required argument '--api-key'`
- 配置错误：`配置文件格式错误: invalid api_key format`
- 权限错误：`权限不足，无法删除文件: /etc/hosts`

**系统错误（详细日志）：**
- Rust 二进制未找到：`FATAL: Cannot find argus-backend binary`
- RPC 连接失败：`ERROR: Lost connection to backend process`
- LLM API 错误：`API Error: 401 Unauthorized`

### 错误码定义

| 错误码 | 含义 | 处理方式 |
|--------|------|----------|
| -32700 | Parse error | 显示原始 JSON，便于调试 |
| -32600 | Invalid Request | 提示版本不匹配 |
| -32601 | Method not found | 提示更新 CLI |
| -32602 | Invalid params | 显示参数错误 |
| -32000 | LLM API error | 提示检查 api_key/base_url |
| -32001 | Session not found | 提示 /sessions 查看列表 |
| -32002 | Tool execution failed | 显示工具输出 |
| -32003 | Rate limited | 提示稍后重试 |

## 输出格式

### 默认文本格式

```
$ argusx agent "帮我重构这个文件"

🤔 正在分析...

📖 读取文件 src/main.rs...
✓ 完成

💭 正在思考如何重构...

✓ 重构完成！
```

### JSON 格式

```bash
$ argusx agent "帮我重构" --json '{"output": "string"}'

{
  "output": "重构完成...",
  "session_id": "abc123",
  "tools_used": 3,
  "duration_ms": 5234
}
```

## 测试策略

### 单元测试

**Node.js 侧：**
- 命令解析测试
- RPC 客户端测试
- 输出格式化测试

**Rust 侧：**
- RPC 协议测试
- RPC 方法测试
- 集成测试

### 集成测试

```typescript
// 端到端测试示例
describe("Agent Execution Flow", () => {
  it("should execute agent with tool calls", async () => {
    const result = await execCLI(
      'argusx agent "列出当前目录文件"'
    );
    expect(result.stdout).toContain("README.md");
  });
});
```

### 手动测试清单

- [ ] 基本对话：`argusx agent "你好"`
- [ ] 会话恢复：`argusx agent "继续" --session <id>`
- [ ] 工具确认：触发危险操作，验证确认框
- [ ] 流式输出：观察实时输出效果
- [ ] 命令系统：测试所有 `/` 命令
- [ ] 错误处理：错误的 api_key、不存在的 session 等
- [ ] JSON 输出：`--json {"output": "string"}`
- [ ] 跨平台：macOS、Linux、Windows

## 构建与发布

### 构建流程

```bash
# 1. 编译 Rust
npm run build:rust
# → 生成 vendor/{target}/argus-backend

# 2. 打包 Node.js
npm pack
# → 生成 @argusx/cli-0.1.0.tgz

# 3. 跨平台构建
npm run build:rust -- --target x86_64-unknown-linux-gnu
npm run build:rust -- --target aarch64-apple-darwin
```

### package.json 脚本

```json
{
  "scripts": {
    "build": "npm run build:rust && npm run build:node",
    "build:rust": "cargo build --release --bin argus-backend",
    "build:node": "tsc",
    "prepublishOnly": "npm run build",
    "test": "vitest"
  }
}
```

## 实现优先级

### Phase 1（核心功能）
1. Node.js CLI 框架
2. JSON-RPC 协议实现（Rust + Node.js）
3. 基本的 `agent.execute` 功能
4. 流式输出显示

### Phase 2（用户体验）
5. 工具确认交互
6. 命令系统（`/` 命令）
7. 配置管理
8. 错误处理优化

### Phase 3（增强功能）
9. 会话管理命令
10. JSON 输出格式
11. 进度显示优化
12. 跨平台构建

## 技术栈

### Node.js 侧
- **commander**: 命令行参数解析
- **inquirer**: 交互式确认
- **ora**: 加载动画
- **chalk**: 彩色输出
- **typescript**: 类型安全

### Rust 侧
- **保持现有依赖**
- 新增：JSON-RPC 库（如 `jsonrpc-rs` 或自实现）

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| JSON-RPC 协议实现复杂度 | 中 | 参考成熟的实现，充分测试 |
| 跨平台二进制分发 | 低 | 使用 npm 包管理平台依赖 |
| 性能开销 | 低 | stdio 通信开销可忽略 |
| 调试困难 | 中 | 添加详细的日志和错误信息 |

## 后续演进

完成这个重构后，可能的后续方向：

1. **Web UI**: 基于 Rust 后端，开发 Web 界面
2. **桌面应用**: 集成到 Tauri 应用
3. **Daemon 模式**: 支持后台服务，多个前端连接
4. **插件系统**: 支持用户自定义工具和命令
