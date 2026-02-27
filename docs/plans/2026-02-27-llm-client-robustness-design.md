# LLM Client 鲁棒性改进设计

## 背景

当前 `bigmodel-api` crate 的 SSE 请求实现存在以下问题：

1. **无重试机制** - 网络错误、429 限流、5xx 服务端错误都直接失败
2. **SSE 解析容错性差** - JSON 解析失败只是 `eprintln!`，不传播错误
3. **无超时控制** - 整体请求 120s 超时，但流式传输没有逐块超时
4. **无 idle timeout** - 如果服务端停止发送数据，连接会一直挂起
5. **无指数退避** - 错误后立即失败，没有重试间隔

## 参考实现

### OpenCode (TypeScript)

- 使用 AI SDK 的 `streamText()` 作为基础
- 完整的指数退避重试 (2s → 4s → 8s...)
- 解析 `retry-after` / `retry-after-ms` 响应头
- 区分可重试/不可重试错误
- AbortSignal 支持取消和超时
- SSE 心跳检测 (10s) 防止代理断连

### Codex (Rust)

- `RetryPolicy` 结构体定义重试策略
- `backoff()` 函数实现指数退避 + 抖动
- `TransportError` / `ApiError` 分层错误处理
- `process_sse()` 使用 `eventsource_stream` crate 解析 SSE
- Idle timeout 检测流停止
- 错误类型自动映射 (context_exceeded, quota_exceeded, rate_limit...)

## 设计方案

### 架构层次

```
agent-turn::adapters::BigModelModelAdapter
    │ implements agent_core::LanguageModel
    │
    └──► llm-client::providers::BigModelHttpClient
              │
              ├── RetryPolicy (指数退避)
              ├── TimeoutConfig (连接/请求/idle)
              ├── LlmError (详细错误类型)
              └── SSE 解析 (idle timeout 检测)
```

### 涉及的 Crate

| Crate | 角色 |
|-------|------|
| **llm-client** (新) | 底层 HTTP 传输 + 重试 + SSE |
| **bigmodel-api** | 只保留类型定义 |
| **agent-turn** | 更新 adapter 使用 llm-client |

> 注：`llm-sdk` / `llm-cli` 为历史包袱，不在本次改进范围内

### 模块结构

```
llm-client/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── config.rs           # RetryPolicy, TimeoutConfig
    ├── error.rs            # LlmError (可重试/不可重试)
    ├── retry.rs            # run_with_retry, backoff
    ├── sse.rs              # SseProcessor with idle timeout
    └── providers/
        ├── mod.rs
        └── bigmodel.rs     # BigModelHttpClient
```

## 核心类型设计

### config.rs

```rust
pub struct RetryPolicy {
    pub max_attempts: u64,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub retry_on: RetryOn,
}

pub struct RetryOn {
    pub retry_429: bool,
    pub retry_5xx: bool,
    pub retry_network: bool,
}

pub struct TimeoutConfig {
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub stream_idle_timeout: Duration,  // 流空闲超时
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(30),
            retry_on: RetryOn {
                retry_429: true,
                retry_5xx: true,
                retry_network: true,
            },
        }
    }
}
```

### error.rs

```rust
pub enum LlmError {
    // 可重试错误
    RateLimit {
        message: String,
        retry_after: Option<Duration>,
    },
    ServerError {
        status: u16,
        message: String,
    },
    NetworkError {
        message: String,
    },
    Timeout,

    // 不可重试错误
    AuthError {
        message: String,
    },
    InvalidRequest {
        message: String,
    },
    ContextOverflow {
        message: String,
    },
    QuotaExceeded {
        message: String,
    },

    // SSE 解析错误
    StreamError {
        message: String,
    },
}

impl LlmError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimit { .. }
                | Self::ServerError { .. }
                | Self::NetworkError { .. }
                | Self::Timeout
        )
    }

    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}
```

### retry.rs

```rust
pub async fn run_with_retry<T, F, Fut>(
    policy: RetryPolicy,
    mut make_request: impl FnMut() -> F,
) -> Result<T, LlmError>
where
    F: Future<Output = Result<T, LlmError>>,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match make_request().await {
            Ok(result) => return Ok(result),
            Err(err) if err.is_retryable() && attempt < policy.max_attempts => {
                let delay = backoff(policy.base_delay, attempt, &err, policy.max_delay);
                tokio::time::sleep(delay).await;
            }
            Err(err) => return Err(err),
        }
    }
}

fn backoff(base: Duration, attempt: u64, err: &LlmError, max_delay: Duration) -> Duration {
    // 优先使用 retry-after 头
    if let Some(retry_after) = err.retry_after() {
        return retry_after.min(max_delay);
    }

    // 指数退避 + 抖动
    use rand::Rng;
    let exp = 2u64.saturating_pow((attempt - 1) as u32);
    let jitter = rand::rng().random_range(0.9..1.1);
    let delay = Duration::from_millis(
        (base.as_millis() as f64 * exp as f64 * jitter) as u64
    );
    delay.min(max_delay)
}
```

### providers/bigmodel.rs

```rust
pub struct BigModelHttpClient {
    http: reqwest::Client,
    config: BigModelConfig,
    retry: RetryPolicy,
    timeout: TimeoutConfig,
}

impl BigModelHttpClient {
    pub fn new(config: BigModelConfig) -> Self { ... }

    /// 非流式请求 (带重试)
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        run_with_retry(self.retry.clone(), || async {
            self.do_chat(&request).await
        }).await
    }

    /// 流式请求 (带重试 + idle timeout)
    pub fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> impl Stream<Item = Result<ChatResponseChunk, LlmError>> {
        // 1. 发起请求 (带重试建立连接)
        // 2. SSE 解析 (带 idle timeout)
        // 3. 错误类型转换
    }

    async fn do_chat(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
        let response = self.http
            .post(&self.config.url)
            .timeout(self.timeout.request_timeout)
            .json(request)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(response.json().await?)
        } else {
            Err(Self::map_error(status, response).await)
        }
    }
}
```

## 改动清单

### 新建 llm-client crate

1. 创建 `llm-client/Cargo.toml`
2. 实现 `config.rs`, `error.rs`, `retry.rs`, `sse.rs`
3. 实现 `providers/bigmodel.rs`

### 修改 bigmodel-api crate

1. 保留类型定义: `ChatRequest`, `ChatResponse`, `ChatResponseChunk`, `Message`, `Content`, `Role`, 等
2. 删除 `client.rs` 中的 `BigModelClient`
3. 更新 `lib.rs` 导出

### 修改 agent-turn crate

1. 更新 `adapters/bigmodel.rs`:
   - `use llm_client::providers::BigModelHttpClient`
   - 更新 `BigModelModelAdapter` 使用新 client
2. 更新 `Cargo.toml` 依赖

## 测试策略

1. **单元测试**: 重试逻辑、退避计算、错误分类
2. **集成测试**: 模拟服务端错误 (429, 500)、网络中断
3. **端到端测试**: 实际 SSE 流测试、idle timeout 触发
