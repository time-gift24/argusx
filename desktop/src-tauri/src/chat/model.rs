use provider::{
    Dialect, ProviderClient, ProviderConfig, Request,
    dialect::openai::schema::{
        common::{
            FunctionCall, FunctionDefinition, Role, Tool as ProviderTool,
            ToolCall as ProviderToolCall,
        },
        request::{ChatCompletionsOptions, ChatMessage},
    },
};
use turn::{LlmStepRequest, ModelRunner, TurnError, TurnMessage};

pub struct ProviderModelRunner {
    client: ProviderClient,
    config: ChatModelConfig,
    tools: Vec<ProviderTool>,
}

impl ProviderModelRunner {
    pub fn from_environment(tool_specs: &[tool::ToolSpec]) -> Result<Self, TurnError> {
        let config = ChatModelConfig::from_environment()?;
        let client = ProviderClient::new(ProviderConfig::new(
            config.dialect,
            config.base_url.clone(),
            config.api_key.clone(),
        ))
        .map_err(|error| TurnError::Runtime(format!("init provider client: {error}")))?;

        Ok(Self {
            client,
            config,
            tools: tool_specs.iter().map(spec_to_provider_tool).collect(),
        })
    }

    fn build_request(&self, request: &LlmStepRequest) -> Request {
        ChatCompletionsOptions {
            model: self.config.model.clone(),
            messages: request
                .messages
                .iter()
                .map(|message| map_turn_message(message.as_ref()))
                .collect(),
            parallel_tool_calls: request.allow_tools.then_some(true),
            stream: Some(true),
            tools: request.allow_tools.then(|| self.tools.clone()),
            ..Default::default()
        }
    }
}

#[async_trait::async_trait]
impl ModelRunner for ProviderModelRunner {
    async fn start(&self, request: LlmStepRequest) -> Result<argus_core::ResponseStream, TurnError> {
        self.client
            .stream(self.build_request(&request))
            .map_err(|error| TurnError::Runtime(error.to_string()))
    }
}

struct ChatModelConfig {
    api_key: String,
    base_url: String,
    dialect: Dialect,
    model: String,
}

impl ChatModelConfig {
    fn from_environment() -> Result<Self, TurnError> {
        let api_key = std::env::var("ARGUSX_OPENAI_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .map_err(|_| {
                TurnError::Runtime(
                    "missing ARGUSX_OPENAI_API_KEY or OPENAI_API_KEY".to_string(),
                )
            })?;
        let base_url = std::env::var("ARGUSX_OPENAI_BASE_URL")
            .or_else(|_| std::env::var("OPENAI_BASE_URL"))
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model = std::env::var("ARGUSX_OPENAI_MODEL")
            .or_else(|_| std::env::var("OPENAI_MODEL"))
            .unwrap_or_else(|_| "gpt-4o-mini".to_string());

        Ok(Self {
            api_key,
            base_url,
            dialect: Dialect::Openai,
            model,
        })
    }
}

fn spec_to_provider_tool(spec: &tool::ToolSpec) -> ProviderTool {
    ProviderTool {
        type_: "function".to_string(),
        function: FunctionDefinition {
            name: spec.name.clone(),
            description: Some(spec.description.clone()),
            parameters: spec.input_schema.clone(),
            strict: None,
            extra: Default::default(),
        },
        extra: Default::default(),
    }
}

fn map_turn_message(message: &TurnMessage) -> ChatMessage {
    match message {
        TurnMessage::User { content } => ChatMessage {
            role: Role::User,
            content: Some(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            extra: Default::default(),
        },
        TurnMessage::AssistantText { content } => ChatMessage {
            role: Role::Assistant,
            content: Some(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            extra: Default::default(),
        },
        TurnMessage::AssistantToolCalls { content, calls } => ChatMessage {
            role: Role::Assistant,
            content: content.as_ref().map(ToString::to_string),
            name: None,
            tool_calls: Some(calls.iter().map(|call| map_tool_call(call.as_ref())).collect()),
            tool_call_id: None,
            extra: Default::default(),
        },
        TurnMessage::ToolResult {
            call_id, content, ..
        } => ChatMessage {
            role: Role::Tool,
            content: Some(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: Some(call_id.to_string()),
            extra: Default::default(),
        },
        TurnMessage::SystemNote { content } => ChatMessage {
            role: Role::Developer,
            content: Some(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            extra: Default::default(),
        },
    }
}

fn map_tool_call(call: &argus_core::ToolCall) -> ProviderToolCall {
    match call {
        argus_core::ToolCall::FunctionCall {
            call_id,
            name,
            arguments_json,
            ..
        } => ProviderToolCall {
            id: call_id.clone(),
            type_: "function".to_string(),
            function: FunctionCall {
                name: name.clone(),
                arguments: arguments_json.clone(),
                extra: Default::default(),
            },
            extra: Default::default(),
        },
        argus_core::ToolCall::Builtin(call) => ProviderToolCall {
            id: call.call_id.clone(),
            type_: "function".to_string(),
            function: FunctionCall {
                name: call.builtin.canonical_name().to_string(),
                arguments: call.arguments_json.clone(),
                extra: Default::default(),
            },
            extra: Default::default(),
        },
        argus_core::ToolCall::Mcp(call) => ProviderToolCall {
            id: call.id.clone(),
            type_: "function".to_string(),
            function: FunctionCall {
                name: call.name.clone().unwrap_or_else(|| "__mcp__call".to_string()),
                arguments: call
                    .arguments_json
                    .clone()
                    .unwrap_or_else(|| "{}".to_string()),
                extra: Default::default(),
            },
            extra: Default::default(),
        },
    }
}
