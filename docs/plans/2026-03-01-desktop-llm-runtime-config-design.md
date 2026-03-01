# Desktop Chat Runtime LLM 配置设计

## 1. 背景与目标

当前 desktop 端在启动时强依赖环境变量（如 `BIGMODEL_API_KEY`）初始化模型客户端，导致：

1. 未配置环境变量时应用无法正常进入可用聊天状态。
2. 用户无法在 UI 内按 provider 动态配置与切换。
3. `model` 选择仅在前端显示，后端并未以 provider+model 作为显式运行参数。

本设计目标：

1. 在 Chat 页面右上角新增配置入口，支持运行时配置（不持久化）。
2. 支持三类 provider：`bigmodel`、`openai`、`anthropic`，全部可配置。
3. 支持每个 provider 的 `apiKey`、`baseUrl`（必填）、`models`（用户自定义列表）和 `headers`（自定义 key/value）。
4. 无可用模型时，模型选项红色提示并禁用 PromptInput。
5. 将 provider+model 作为 turn 级参数传递到后端与模型层。

## 2. 非目标

1. 本次不做配置持久化（不写磁盘，不写数据库）。
2. 本次不引入新的设置页面，入口仅在 Chat 页右上角。
3. 本次不移除 BigModel，三种 provider 全部保留。

## 3. 方案对比与选择

### 方案 A：仅后端全局 runtime 热更新

- 思路：保存配置后重建一次全局 runtime，继续只用默认 provider。
- 优点：改动小。
- 缺点：无法表达每次 turn 的 provider/model 选择；并发会话语义不清晰。

### 方案 B（采用）：turn 级 provider/model 显式传参

- 思路：前端在发消息时传入 provider+model，后端按 turn 校验与路由，模型层按请求动态选 adapter。
- 优点：语义正确，支持并发会话独立选择；与 UI 模型选择一致。
- 缺点：需跨 `desktop`、`agent-core`、`agent-turn`、`llm-client` 改动。

### 方案 C：完全网关化

- 思路：desktop 统一走 llm-gateway，由 gateway 承担 provider 配置。
- 优点：长期架构清晰。
- 缺点：本次改造范围过大，不满足快速落地。

## 4. 总体架构

```text
Chat UI
  ├─ Runtime Config Dialog (in-memory)
  ├─ Available Models Derivation
  └─ start_agent_turn(provider, model, input, ...)
            │
            ▼
Tauri Desktop Backend
  ├─ AppState.llm_runtime_config (RwLock)
  ├─ get/set config commands
  ├─ list_available_models command
  └─ start_agent_turn validation + turn options
            │
            ▼
agent-core / agent-turn
  ├─ TurnRequest{provider, model}
  ├─ ModelRequest{provider, model}
  └─ RoutedModelAdapter: dispatch by provider
            │
            ▼
llm-client
  ├─ bigmodel adapter
  ├─ openai adapter
  └─ anthropic adapter
```

## 5. 数据模型

### 5.1 前后端共识结构

```ts
type ProviderId = "bigmodel" | "openai" | "anthropic";

type HeaderPair = {
  key: string;
  value: string;
};

type ProviderRuntimeConfig = {
  apiKey: string;      // required
  baseUrl: string;     // required
  models: string[];    // required, >= 1
  headers: HeaderPair[];
};

type LlmRuntimeConfig = {
  defaultProvider?: ProviderId;
  providers: Record<ProviderId, ProviderRuntimeConfig>;
};
```

### 5.2 可用模型投影

```ts
type AvailableModel = {
  provider: ProviderId;
  model: string;
};
```

生成规则：仅包含配置完整且 `models` 非空的 provider。

## 6. API / 命令设计

在 `desktop/src-tauri/src/lib.rs` 新增命令：

1. `get_llm_runtime_config() -> LlmRuntimeConfig`
2. `set_llm_runtime_config(payload: LlmRuntimeConfig) -> LlmRuntimeConfig`
3. `list_available_models() -> Vec<AvailableModel>`

调整现有命令：

- `start_agent_turn(payload)` 增加：
  - `provider: ProviderId`（必填）
  - `model: String`（改为必填语义，且必须属于该 provider 的 models）

## 7. 前端交互设计

### 7.1 Chat 右上角配置按钮

1. 在 Chat 页右上角新增配置按钮（`Settings` 图标）。
2. 按钮有持续轻动画（呼吸/脉冲），hover 强化过渡。
3. 点击打开 `Dialog` 弹窗。

### 7.2 配置弹窗

1. 使用 `Tabs` 分为 `BigModel`、`OpenAI`、`Anthropic`。
2. 每个 Tab 包含：
   - API Key 输入框（必填）
   - Base URL 输入框（必填）
   - Models 可增删输入行（至少 1 条）
   - Headers 可增删 key/value 行
3. 保存后立即调用 `set_llm_runtime_config` 并刷新可用模型列表。

### 7.3 PromptInput 约束

1. 当 `availableModels` 为空时：
   - 模型区域显示红色文案：`No available models. Please configure provider settings.`
   - PromptInput 输入框、提交按钮、附件按钮禁用。
2. 当有可用模型时：
   - 恢复可编辑。
   - 若当前选中模型失效，自动回退到第一个可用模型。

## 8. 后端行为与校验

### 8.1 set 配置时规范化

1. `apiKey` / `baseUrl` trim 后不可为空。
2. `models` 做 trim、去空、去重。
3. `headers` 丢弃空 key；保留最后一个同名 key（或按实现约定去重）。
4. `defaultProvider` 若不可用，则自动修正为第一个可用 provider（若存在）。

### 8.2 start turn 时校验

1. provider 必须存在且配置完整。
2. model 必须属于该 provider 的 `models`。
3. 校验失败返回业务错误，不启动 turn。

## 9. 模型层改造（turn 级 provider 路由）

### 9.1 agent-core

- `TurnRequest` 新增 provider/model 字段。
- `ModelRequest` 新增 provider/model 字段。

### 9.2 agent-turn

1. 将 `convert_model_request` 改为优先使用 request 携带的 model/provider。
2. 引入 `RoutedModelAdapter`（或等价实现）按 provider 分发。

### 9.3 llm-client

1. 保留 `bigmodel` adapter。
2. 新增 `openai` 与 `anthropic` provider adapter。
3. 支持 per-provider:
   - `base_url`
   - `api_key`
   - `headers`
4. 增加显式按 adapter id 调用的接口（避免只依赖 default adapter）。

## 10. 错误处理

1. 配置保存失败：前端弹窗内展示错误，表单值保留。
2. 启动 turn 失败：在 chat 输入区域显示失败提示，session 状态回退到 `wait-input`。
3. Provider HTTP 错误：继续映射到 `LlmError`，保持现有重试/超时策略。

## 11. 测试与验收

### 11.1 前端

1. 无可用模型时，PromptInput 全禁用且出现红色提示。
2. 保存配置后，模型列表即时更新。
3. 配置按钮动画可见且不影响可访问性（支持 reduced motion）。

### 11.2 后端

1. 无环境变量时 desktop 可启动。
2. `set/get/list` 命令的结构化测试。
3. `start_agent_turn` provider/model 校验测试。

### 11.3 集成

1. 以 `openai`、`anthropic`、`bigmodel` 各配置一组 mock 参数，验证都可完成 turn 启动流程。
2. 切换 provider/model 不需要重启应用。
3. 重启应用后配置消失（符合本次“不持久化”约束）。

## 12. 风险与缓解

1. 跨 crate 改动较广：先引入兼容字段与默认分支，分步迁移调用点。
2. provider 协议差异（OpenAI/Anthropic）可能导致映射差异：先保证最小可用能力（text + stream + tools 可选），后续增量补齐。
3. UI 和后端模型可用性不一致：统一以 `list_available_models` 为单一真源。

## 13. 交付物

1. Chat 页右上角配置按钮（含动画）与配置弹窗。
2. PromptInput 可用性联动（红色提示 + 禁用态）。
3. Tauri runtime 配置命令与内存态配置。
4. turn 级 provider/model 传递链路。
5. llm-client 的三 provider 实现与配置能力。

---

Created: 2026-03-01
Status: Approved for implementation planning
