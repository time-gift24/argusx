# Node.js + Rust CLI 重构实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**目标:** 将 Rust CLI 重构为 Node.js 前端 + Rust 后端架构，通过 JSON-RPC over stdio 通信，提升开发体验和用户体验。

**架构:**
- Node.js CLI (`argusx`) 负责参数解析、用户交互、输出格式化
- Rust 后端 (`argus-backend`) 保持核心 agent 逻辑，提供 JSON-RPC 接口
- 通过 stdin/stdout 进行 JSON-RPC 2.0 通信

**技术栈:**
- Node.js: TypeScript, commander, inquirer, ora, chalk
- Rust: 保持现有依赖，新增 JSON-RPC 实现

---

## Phase 1: 基础设施（JSON-RPC 协议）

### Task 1: 创建 Rust JSON-RPC 协议层

**目标:** 定义 JSON-RPC 消息类型和协议处理逻辑

**Files:**
- Create: `agent-cli/src/rpc/mod.rs`
- Create: `agent-cli/src/rpc/protocol.rs`
- Create: `agent-cli/src/rpc/protocol_test.rs`

**Step 1: 编写协议类型的单元测试**

```rust
// agent-cli/src/rpc/protocol_test.rs
#[cfg(test)]
mod tests {
    use super::super::protocol::*;
    use serde_json::json;

    #[test]
    fn test_parse_valid_request() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
        let req: Request = serde_json::from_str(raw).unwrap();
        assert_eq!(req.method, "test");
    }

    #[test]
    fn test_serialize_response() {
        let resp = Response::success(1, json!({"result": "ok"}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"result\":"));
    }
}
```

**Step 2: 运行测试验证失败**

Run: `cargo test -p agent-cli rpc`
Expected: FAIL with "module not found" or similar

**Step 3: 实现 JSON-RPC 协议类型**

```rust
// agent-cli/src/rpc/protocol.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}

impl Response {
    pub fn success(id: u64, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: u64, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(Error { code, message }),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Error {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
}
```

**Step 4: 创建模块导出**

```rust
// agent-cli/src/rpc/mod.rs
pub mod protocol;

pub use protocol::{Error, Notification, Request, Response};
```

**Step 5: 运行测试验证通过**

Run: `cargo test -p agent-cli rpc`
Expected: PASS

**Step 6: 提交**

```bash
git add agent-cli/src/rpc/
git commit -m "feat(rpc): add JSON-RPC protocol types

- Define Request, Response, Notification types
- Implement success/error response builders
- Add unit tests for serialization"
```

---

### Task 2: 实现 JSON-RPC Server

**目标:** 创建 Rust 侧的 JSON-RPC 服务器，从 stdin 读取请求，写入响应到 stdout

**Files:**
- Create: `agent-cli/src/rpc/server.rs`
- Modify: `agent-cli/src/rpc/mod.rs`

**Step 1: 编写服务器集成测试**

```rust
// agent-cli/tests/rpc_server_test.rs
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn test_server_handles_request() {
    let mut child = Command::new("cargo")
        .args(["run", "-p", "agent-cli", "--", "--mode", "rpc-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn");

    // 发送测试请求
    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"ping","params":{{}}}}"#).unwrap();

    // 读取响应
    let stdout = BufReader::new(child.stdout.as_mut().expect("Failed to open stdout"));
    let line = stdout.lines().next().unwrap().unwrap();

    assert!(line.contains("\"result\""));

    child.kill().ok();
}
```

**Step 2: 运行测试验证失败**

Run: `cargo test -p agent-cli rpc_server`
Expected: FAIL with "unknown mode" or similar

**Step 3: 实现 JSON-RPC 服务器**

```rust
// agent-cli/src/rpc/server.rs
use crate::rpc::protocol::{Request, Response};
use anyhow::Result;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct RpcServer {
    handlers: Arc<Vec<String>>,
}

impl RpcServer {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(vec!["ping".to_string()]),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let reader = BufReader::new(stdin);
        let mut writer = stdout.lock();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(req) = serde_json::from_str::<Request>(&line) {
                let resp = self.handle_request(req).await?;
                writeln!(writer, "{}", serde_json::to_string(&resp)?)?;
                writer.flush()?;
            }
        }

        Ok(())
    }

    async fn handle_request(&self, req: Request) -> Result<Response> {
        match req.method.as_str() {
            "ping" => Ok(Response::success(req.id, serde_json::json!({"pong": true}))),
            _ => Ok(Response::error(req.id, -32601, "Method not found".to_string())),
        }
    }
}
```

**Step 4: 更新模块导出**

```rust
// agent-cli/src/rpc/mod.rs
pub mod protocol;
pub mod server;

pub use protocol::{Error, Notification, Request, Response};
pub use server::RpcServer;
```

**Step 5: 修改 main.rs 支持 rpc-server 模式**

```rust
// agent-cli/src/main.rs
use agent_cli::rpc::RpcServer;
use clap::Parser;

#[derive(Parser)]
struct CliArgs {
    #[arg(long)]
    mode: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    if args.mode.as_deref() == Some("rpc-server") {
        let server = RpcServer::new();
        server.run().await?;
        return Ok(());
    }

    // 原有的 TUI 逻辑保持不变...
    Ok(())
}
```

**Step 6: 运行测试验证通过**

Run: `cargo test -p agent-cli rpc_server`
Expected: PASS

**Step 7: 提交**

```bash
git add agent-cli/src/rpc/server.rs agent-cli/src/main.rs agent-cli/tests/
git commit -m "feat(rpc): implement JSON-RPC server

- Add RpcServer that reads from stdin, writes to stdout
- Implement ping handler for testing
- Add --mode rpc-server flag to main.rs
- Add integration test for server"
```

---

### Task 3: 创建 Node.js CLI 框架

**目标:** 搭建 Node.js 项目基础结构，实现二进制定位和 spawn 逻辑

**Files:**
- Create: `agent-js/package.json`
- Create: `agent-js/bin/argusx.js`
- Create: `agent-js/tsconfig.json`

**Step 1: 创建 package.json**

```json
{
  "name": "@argusx/cli",
  "version": "0.1.0",
  "license": "MIT",
  "bin": {
    "argusx": "bin/argusx.js"
  },
  "type": "module",
  "engines": {
    "node": ">=16"
  },
  "files": [
    "bin",
    "vendor",
    "dist"
  ],
  "scripts": {
    "build": "tsc",
    "build:rust": "cargo build --release --bin argus-backend",
    "prepublishOnly": "npm run build"
  },
  "devDependencies": {
    "@types/node": "^20.0.0",
    "typescript": "^5.0.0"
  },
  "dependencies": {
    "commander": "^12.0.0"
  }
}
```

**Step 2: 创建 TypeScript 配置**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "moduleResolution": "node",
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules"]
}
```

**Step 3: 实现主入口（参考 codex.js）**

```javascript
// agent-js/bin/argusx.js
#!/usr/bin/env node
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// 平台检测
const PLATFORM_TARGETS = {
  "darwin-arm64": "aarch64-apple-darwin",
  "darwin-x64": "x86_64-apple-darwin",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "win32-arm64": "aarch64-pc-windows-msvc",
  "win32-x64": "x86_64-pc-windows-msvc",
};

const platform = process.platform;
const arch = process.arch;
const targetKey = `${platform}-${arch}`;

const targetTriple = PLATFORM_TARGETS[targetKey];
if (!targetTriple) {
  console.error(`Unsupported platform: ${platform} (${arch})`);
  process.exit(1);
}

// 二进制定位
const binaryName = platform === "win32" ? "argus-backend.exe" : "argus-backend";
const vendorRoot = path.join(__dirname, "..", "vendor", targetTriple);
const binaryPath = path.join(vendorRoot, "release", binaryName);

if (!existsSync(binaryPath)) {
  console.error(`Backend binary not found: ${binaryPath}`);
  console.error(`Run: npm run build:rust`);
  process.exit(1);
}

// Spawn 后端进程
const args = ["--mode", "rpc-server", ...process.argv.slice(2)];
const child = spawn(binaryPath, args, {
  stdio: ["inherit", "pipe", "inherit"],
});

child.on("error", (err) => {
  console.error("Failed to start backend:", err);
  process.exit(1);
});

// 简单的 stdout 处理（后续会改为 JSON-RPC）
child.stdout.on("data", (data) => {
  process.stdout.write(data);
});

child.on("exit", (code) => {
  process.exit(code ?? 1);
});
```

**Step 4: 创建目录结构**

Run: `mkdir -p agent-js/src agent-js/bin agent-js/vendor`

**Step 5: 设置可执行权限**

Run: `chmod +x agent-js/bin/argusx.js`

**Step 6: 提交**

```bash
git add agent-js/
git commit -m "feat(node): create Node.js CLI framework

- Add package.json with TypeScript setup
- Implement argusx.js entry point with platform detection
- Add binary location logic similar to codex-cli
- Add build:rust script to compile backend"
```

---

### Task 4: 实现 Node.js JSON-RPC 客户端

**目标:** 实现 JSON-RPC 客户端，与 Rust 后端通信

**Files:**
- Create: `agent-js/src/rpc/types.ts`
- Create: `agent-js/src/rpc/client.ts`
- Create: `agent-js/tests/rpc-client.test.ts`

**Step 1: 定义类型**

```typescript
// agent-js/src/rpc/types.ts
export interface JsonRpcRequest {
  jsonrpc: "2.0";
  id: number;
  method: string;
  params?: unknown;
}

export interface JsonRpcResponse {
  jsonrpc: "2.0";
  id: number;
  result?: unknown;
  error?: {
    code: number;
    message: string;
  };
}

export interface JsonRpcNotification {
  jsonrpc: "2.0";
  method: string;
  params: unknown;
}
```

**Step 2: 实现 RPC 客户端**

```typescript
// agent-js/src/rpc/client.ts
import { ChildProcess } from "node:child_process";
import { createInterface } from "node:readline";
import type { JsonRpcNotification, JsonRpcRequest, JsonRpcResponse } from "./types.js";

export class RpcClient {
  private id = 0;
  private pending = new Map<number, (resp: JsonRpcResponse) => void>();
  private rl: ReturnType<typeof createInterface>;

  constructor(private child: ChildProcess) {
    this.rl = createInterface({
      input: this.child.stdout!,
      crlfDelay: Infinity,
    });

    this.rl.on("line", (line) => {
      try {
        const resp: JsonRpcResponse = JSON.parse(line);
        const resolver = this.pending.get(resp.id);
        if (resolver) {
          resolver(resp);
          this.pending.delete(resp.id);
        }
      } catch {
        // 忽略非 JSON 输出
      }
    });
  }

  async call(method: string, params?: unknown): Promise<unknown> {
    const id = ++this.id;
    const request: JsonRpcRequest = {
      jsonrpc: "2.0",
      id,
      method,
      params,
    };

    return new Promise((resolve, reject) => {
      this.pending.set(id, (resp) => {
        if (resp.error) {
          reject(new Error(`${resp.error.code}: ${resp.error.message}`));
        } else {
          resolve(resp.result);
        }
      });

      this.child.stdin!.write(JSON.stringify(request) + "\n");

      // 超时处理
      setTimeout(() => {
        if (this.pending.has(id)) {
          this.pending.delete(id);
          reject(new Error("RPC timeout"));
        }
      }, 30000);
    });
  }

  onNotification(handler: (notif: JsonRpcNotification) => void) {
    this.rl.on("line", (line) => {
      try {
        const notif: JsonRpcNotification = JSON.parse(line);
        if (!notif.id) {
          handler(notif);
        }
      } catch {
        // 忽略非 JSON 输出
      }
    });
  }
}
```

**Step 3: 编写客户端测试**

```typescript
// agent-js/tests/rpc-client.test.ts
import { RpcClient } from "../src/rpc/client.js";
import { spawn } from "node:child_process";

describe("RpcClient", () => {
  it("should call ping method", async () => {
    const child = spawn("cargo", [
      "run",
      "-p",
      "agent-cli",
      "--",
      "--mode",
      "rpc-server",
    ]);

    const client = new RpcClient(child);
    const result = await client.call("ping", {});

    expect(result).toEqual({ pong: true });

    child.kill();
  });
});
```

**Step 4: 安装测试依赖**

Run: `cd agent-js && npm install -D vitest @types/node`

**Step 5: 运行测试验证通过**

Run: `cd agent-js && npm test`
Expected: PASS

**Step 6: 提交**

```bash
git add agent-js/src/rpc/ agent-js/tests/
git commit -m "feat(node): implement JSON-RPC client

- Add RpcClient class with call() and onNotification()
- Implement request/response handling over stdin/stdout
- Add timeout and error handling
- Add unit tests with vitest"
```

---

## Phase 2: 核心 Agent 功能

### Task 5: 实现 agent.execute RPC 方法

**目标:** 在 Rust 侧实现 agent 执行逻辑

**Files:**
- Modify: `agent-cli/src/rpc/server.rs`
- Modify: `agent-cli/src/rpc/handlers.rs` (create)
- Create: `agent-cli/tests/agent_execute_test.rs`

**Step 1: 编写测试**

```rust
// agent-cli/tests/agent_execute_test.rs
#[test]
fn test_agent_execute_returns_output() {
    // 集成测试：调用 agent.execute 并验证返回值
}
```

**Step 2: 实现 handler**

```rust
// agent-cli/src/rpc/handlers.rs
use crate::rpc::protocol::Response;
use agent_core::LanguageModel;
use anyhow::Result;
use serde_json::json;

pub struct AgentHandler {
    // agent 实例
}

impl AgentHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execute(&self, params: serde_json::Value) -> Result<Response> {
        let prompt = params["prompt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing prompt"))?;

        // TODO: 实际的 agent 调用逻辑
        // 这里先用简单实现测试通信
        Ok(Response::success(
            1,
            json!({
                "output": format!("Echo: {}", prompt),
                "session_id": "test-123"
            }),
        ))
    }
}
```

**Step 3: 集成到 server**

```rust
// agent-cli/src/rpc/server.rs
use super::handlers::AgentHandler;

pub struct RpcServer {
    agent: AgentHandler,
}

impl RpcServer {
    pub fn new() -> Self {
        Self {
            agent: AgentHandler::new(),
        }
    }

    async fn handle_request(&self, req: Request) -> Result<Response> {
        match req.method.as_str() {
            "ping" => Ok(Response::success(req.id, serde_json::json!({"pong": true}))),
            "agent.execute" => self.agent.execute(req.params).await,
            _ => Ok(Response::error(req.id, -32601, "Method not found".to_string())),
        }
    }
}
```

**Step 4: 运行测试**

Run: `cargo test -p agent-cli agent_execute`
Expected: PASS

**Step 5: 提交**

```bash
git add agent-cli/src/rpc/handlers.rs
git commit -m "feat(rpc): implement agent.execute handler

- Add AgentHandler to execute agent requests
- Integrate into RpcServer
- Return placeholder response for testing
- Add tests for agent.execute method"
```

---

### Task 6: 实现流式输出

**目标:** 支持流式输出 LLM 响应

**Files:**
- Modify: `agent-cli/src/rpc/server.rs`
- Modify: `agent-js/src/rpc/client.ts`
- Create: `agent-js/src/ui/formatting.ts`

**Step 1: Rust 侧发送流式通知**

```rust
// agent-cli/src/rpc/server.rs
use crate::rpc::protocol::Notification;

impl RpcServer {
    // 在执行过程中发送流式更新
    async fn stream_output(&self, message: &str) -> Result<()> {
        let notif = Notification {
            jsonrpc: "2.0".to_string(),
            method: "content.delta".to_string(),
            params: serde_json::json!({
                "content": message,
                "reasoning": null
            }),
        };

        let stdout = io::stdout();
        let mut writer = stdout.lock();
        writeln!(writer, "{}", serde_json::to_string(&notif)?)?;
        writer.flush()?;

        Ok(())
    }
}
```

**Step 2: Node.js 侧处理流式通知**

```typescript
// agent-js/src/ui/formatting.ts
import type { JsonRpcNotification } from "../rpc/types.js";

export function displayStream(notif: JsonRpcNotification) {
  if (notif.method === "content.delta") {
    const params = notif.params as { content: string; reasoning: string | null };
    process.stdout.write(params.content);
  }
}
```

**Step 3: 更新客户端**

```typescript
// agent-js/src/rpc/client.ts
import { displayStream } from "../ui/formatting.js";

// 在 constructor 中添加
this.onNotification((notif) => {
  displayStream(notif);
});
```

**Step 4: 测试流式输出**

```bash
# 手动测试
cargo run -p agent-cli -- --mode rpc-server
echo '{"jsonrpc":"2.0","method":"content.delta","params":{"content":"Hello "}}' | \
  node agent-js/bin/argusx.js
```

**Step 5: 提交**

```bash
git add agent-cli/src/rpc/server.rs agent-js/src/ui/formatting.ts
git commit -m "feat: implement streaming output

- Add content.delta notification on Rust side
- Implement displayStream() in Node.js
- Handle streaming notifications in RpcClient
- Test manual streaming flow"
```

---

## Phase 3: 用户界面增强

### Task 7: 实现工具确认功能

**目标:** 当 Agent 要执行危险工具时，弹出确认框

**Files:**
- Create: `agent-js/src/ui/prompts.ts`
- Modify: `agent-js/src/rpc/client.ts`

**Step 1: 实现确认提示**

```typescript
// agent-js/src/ui/prompts.ts
import readline from "node:readline";

export async function confirmAction(
  tool: string,
  params: Record<string, unknown>,
  risk: string
): Promise<boolean> {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  const riskEmoji = risk === "high" ? "⚠️ " : "";
  const question = `\n${riskEmoji}执行工具: ${tool}\n参数: ${JSON.stringify(params, null, 2)}\n确认执行? [y/N] `;

  return new Promise((resolve) => {
    rl.question(question, (answer) => {
      rl.close();
      resolve(answer.toLowerCase() === "y" || answer.toLowerCase() === "yes");
    });
  });
}
```

**Step 2: 处理工具确认通知**

```typescript
// agent-js/src/rpc/client.ts
import { confirmAction } from "../ui/prompts.js";

// 在 onNotification 中添加
this.onNotification(async (notif) => {
  if (notif.method === "tool.confirm") {
    const params = notif.params as {
      id: number;
      tool: string;
      params: Record<string, unknown>;
      risk: string;
    };

    const approved = await confirmAction(
      params.tool,
      params.params,
      params.risk
    );

    const method = approved ? "tool.approve" : "tool.reject";
    await this.call(method, { id: params.id });
  }
});
```

**Step 3: Rust 侧等待确认**

```rust
// agent-cli/src/rpc/handlers.rs
use tokio::sync::oneshot;

pub struct ToolConfirmation {
    pub approved: oneshot::Sender<bool>,
}

impl AgentHandler {
    pub async fn execute_with_tools(&self) -> Result<()> {
        // 当需要确认时
        let (tx, rx) = oneshot::channel();
        // 发送 tool.confirm 通知...

        let approved = rx.await?;
        if approved {
            // 执行工具
        }
        Ok(())
    }
}
```

**Step 4: 手动测试**

```bash
# 触发确认流程
```

**Step 5: 提交**

```bash
git add agent-js/src/ui/prompts.ts
git commit -m "feat: add tool confirmation prompts

- Implement confirmAction() for user confirmation
- Handle tool.confirm notifications
- Send tool.approve/reject RPC calls
- Add approval wait logic on Rust side"
```

---

### Task 8: 实现命令系统

**目标:** 支持 `/` 前缀的命令

**Files:**
- Create: `agent-js/src/cli/commands.ts`
- Modify: `agent-js/bin/argusx.js`

**Step 1: 定义命令**

```typescript
// agent-js/src/cli/commands.ts
import type { RpcClient } from "../rpc/client.js";

interface Command {
  description: string;
  handler: (args: string[], client: RpcClient) => Promise<void>;
}

const COMMANDS: Record<string, Command> = {
  help: {
    description: "显示帮助信息",
    handler: async () => {
      console.log(`
可用命令:
  /help, /?          显示帮助
  /sessions          列出会话
  /new               新建会话
  /exit, /quit       退出
      `);
    },
  },

  sessions: {
    description: "列出所有会话",
    handler: async (_args, client) => {
      const result = (await client.call("session.list", {})) as {
        sessions: unknown[];
      };
      console.log("会话列表:", result.sessions);
    },
  },

  new: {
    description: "创建新会话",
    handler: async (_args, client) => {
      await client.call("agent.execute", { prompt: "", session_id: null });
    },
  },

  exit: {
    description: "退出",
    handler: async () => {
      process.exit(0);
    },
  },
};

export async function handleCommand(
  input: string,
  client: RpcClient
): Promise<boolean> {
  if (!input.startsWith("/")) {
    return false;
  }

  const [cmd, ...args] = input.slice(1).split(/\s+/);
  const command = COMMANDS[cmd];

  if (command) {
    await command.handler(args, client);
    return true;
  }

  console.log(`未知命令: ${cmd}`);
  console.log('输入 /help 查看可用命令');
  return true;
}

export { COMMANDS };
```

**Step 2: 集成到主循环**

```typescript
// agent-js/bin/argusx.js
import { handleCommand } from "../dist/cli/commands.js";

// 在主循环中
const readline = require("node:readline");
const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
});

rl.on("line", async (line) => {
  const isCommand = await handleCommand(line, client);
  if (!isCommand) {
    // 发送到 agent
    await client.call("agent.execute", { prompt: line });
  }
});
```

**Step 3: 测试命令**

```bash
node agent-js/bin/argusx.js
> /help
> /sessions
```

**Step 4: 提交**

```bash
git add agent-js/src/cli/commands.ts
git commit -m "feat: implement command system

- Add /help, /sessions, /new, /exit commands
- Implement handleCommand() function
- Integrate command parsing into REPL
- Add command descriptions"
```

---

## Phase 4: 完善与优化

### Task 9: 实现配置管理

**目标:** 支持配置文件和环境变量

**Files:**
- Create: `agent-js/src/config/loader.ts`
- Create: `agent-js/src/config/schema.ts`

**Step 1: 定义配置结构**

```typescript
// agent-js/src/config/schema.ts
export interface Config {
  apiKey?: string;
  baseUrl?: string;
  model?: string;
  storeDir?: string;
}

export const DEFAULT_CONFIG: Config = {
  baseUrl: "https://open.bigmodel.cn/api/paas/v4",
  model: "glm-5",
};
```

**Step 2: 实现配置加载**

```typescript
// agent-js/src/config/loader.ts
import fs from "node:fs/promises";
import path from "node:path";
import { DEFAULT_CONFIG, type Config } from "./schema.js";

export async function loadConfig(): Promise<Config> {
  const configPath = path.join(process.env.HOME || "", ".argusx", "config.json");

  try {
    const content = await fs.readFile(configPath, "utf-8");
    const userConfig = JSON.parse(content);
    return { ...DEFAULT_CONFIG, ...userConfig };
  } catch {
    return DEFAULT_CONFIG;
  }
}

export function mergeWithEnv(config: Config): Config {
  return {
    ...config,
    apiKey: config.apiKey || process.env.BIGMODEL_API_KEY,
    baseUrl: config.baseUrl || process.env.BIGMODEL_BASE_URL,
  };
}
```

**Step 3: 在启动时加载配置**

```typescript
// agent-js/bin/argusx.js
import { loadConfig, mergeWithEnv } from "../dist/config/loader.js";

const config = mergeWithEnv(await loadConfig());
```

**Step 4: 添加 /config 命令**

```typescript
// agent-js/src/cli/commands.ts
config: {
  description: "查看配置",
  handler: async () => {
    const config = await loadConfig();
    console.log(JSON.stringify(config, null, 2));
  },
}
```

**Step 5: 提交**

```bash
git add agent-js/src/config/
git commit -m "feat: add configuration management

- Define Config interface and defaults
- Implement loadConfig() from ~/.argusx/config.json
- Merge with environment variables
- Add /config command"
```

---

### Task 10: 实现错误处理

**目标:** 完善错误提示和处理

**Files:**
- Create: `agent-js/src/errors/handler.ts`
- Modify: `agent-js/src/rpc/client.ts`

**Step 1: 定义错误处理器**

```typescript
// agent-js/src/errors/handler.ts
export function handleRpcError(error: { code: number; message: string }): void {
  const errorMap: Record<number, string> = {
    [-32700]: "解析错误",
    [-32600]: "无效请求",
    [-32601]: "方法不存在",
    [-32602]: "参数错误",
    [-32000]: "LLM API 错误",
    [-32001]: "会话不存在",
    [-32002]: "工具执行失败",
    [-32003]: "请求过于频繁",
  };

  const message = errorMap[error.code] || error.message;
  console.error(`❌ 错误: ${message}`);

  if (error.code === -32000) {
    console.error("提示: 请检查 BIGMODEL_API_KEY 环境变量");
  }
}

export function checkBinaryExists(binaryPath: string): never {
  const fs = require("node:fs");
  if (!fs.existsSync(binaryPath)) {
    console.error(`❌ 找不到后端二进制: ${binaryPath}`);
    console.error(`请运行: npm run build:rust`);
    process.exit(1);
  }
}
```

**Step 2: 集成到客户端**

```typescript
// agent-js/src/rpc/client.ts
import { handleRpcError } from "../errors/handler.js";

// 在 call() 方法中
if (resp.error) {
  handleRpcError(resp.error);
  reject(new Error(`${resp.error.code}: ${resp.error.message}`));
}
```

**Step 3: 添加二进制检查**

```typescript
// agent-js/bin/argusx.js
import { checkBinaryExists } from "../dist/errors/handler.js";

checkBinaryExists(binaryPath);
```

**Step 4: 测试错误处理**

```bash
# 测试各种错误场景
```

**Step 5: 提交**

```bash
git add agent-js/src/errors/
git commit -m "feat: add error handling

- Implement handleRpcError() with user-friendly messages
- Add checkBinaryExists() for setup validation
- Integrate error handlers into RpcClient
- Provide actionable error hints"
```

---

### Task 11: 实现 JSON 输出格式

**目标:** 支持 `--json {schema}` 参数

**Files:**
- Modify: `agent-js/src/cli/commands.ts`
- Modify: `agent-cli/src/rpc/handlers.rs`

**Step 1: 解析 --json 参数**

```typescript
// agent-js/src/cli/commands.ts
export function parseJsonOption(args: string[]): { schema: unknown } | null {
  const jsonIndex = args.indexOf("--json");
  if (jsonIndex === -1) {
    return null;
  }

  const schemaStr = args[jsonIndex + 1];
  try {
    const schema = JSON.parse(schemaStr);
    return { schema };
  } catch {
    console.error("无效的 JSON schema");
    return null;
  }
}
```

**Step 2: 传递到后端**

```typescript
// 在调用 agent.execute 时
const jsonOption = parseJsonOption(process.argv);
const result = await client.call("agent.execute", {
  prompt,
  jsonSchema: jsonOption?.schema,
});
```

**Step 3: Rust 侧处理结构化输出**

```rust
// agent-cli/src/rpc/handlers.rs
pub async fn execute(&self, params: serde_json::Value) -> Result<Response> {
    let prompt = params["prompt"].as_str().unwrap();
    let json_schema = params.get("jsonSchema");

    if let Some(schema) = json_schema {
        // 使用 LLM 的结构化输出
        let output = self.agent.structured(prompt, schema).await?;
        Ok(Response::success(1, output))
    } else {
        // 普通输出
        let output = self.agent.chat(prompt).await?;
        Ok(Response::success(1, json!({"output": output.to_string()})))
    }
}
```

**Step 4: 测试 JSON 输出**

```bash
argusx agent "当前时间" --json '{"output":"string"}'
```

**Step 5: 提交**

```bash
git commit -m "feat: add JSON output format support

- Parse --json {schema} argument
- Pass schema to backend agent
- Implement structured output in Rust handler
- Test JSON response format"
```

---

## 测试清单

完成所有任务后，进行以下测试：

### 单元测试
```bash
# Rust 侧
cargo test -p agent-cli

# Node.js 侧
cd agent-js && npm test
```

### 集成测试
```bash
# 基本对话
argusx agent "你好"

# 会话管理
argusx agent "测试" --session abc123
argusx /sessions

# 工具确认
argusx agent "删除测试文件.txt"

# JSON 输出
argusx agent "总结" --json '{"summary":"string"}'

# 错误处理
BIGMODEL_API_KEY=invalid argusx agent "测试"
```

### 跨平台测试
- [ ] macOS (x64, ARM64)
- [ ] Linux (x64, ARM64)
- [ ] Windows (x64)

---

## 发布清单

- [ ] 更新 README.md
- [ ] 添加使用示例
- [ ] 构建 Release 二进制
- [ ] 发布到 npm
- [ ] 创建 Homebrew formula (可选)

---

## 预计工作量

- Phase 1: 1 天（基础设施）
- Phase 2: 1 天（核心功能）
- Phase 3: 0.5 天（UI 增强）
- Phase 4: 0.5 天（完善优化）

**总计: 3 天**

---

## 参考

- codex-cli 实现: `.vendor/codex/codex-cli/`
- JSON-RPC 2.0 规范: https://www.jsonrpc.org/specification
- 设计文档: `docs/plans/2026-03-01-nodejs-rust-cli-refactor-design.md`
