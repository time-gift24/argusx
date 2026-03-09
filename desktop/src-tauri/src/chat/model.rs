use std::{env, path::PathBuf};

use agent::AgentToolSurface;
use async_trait::async_trait;
use provider::{
    Dialect, ProviderClient, ProviderConfig, ProviderDevOptions, ReplayTiming, Request,
    dialect::openai::schema::{
        common::{
            FunctionCall, FunctionDefinition, Role, Tool as ProviderTool,
            ToolCall as ProviderToolCall, ToolChoice,
        },
        request::{ChatCompletionsOptions, ChatMessage},
    },
};
use serde_json::{Map, Value};
use tool::ToolSpec;
use turn::{LlmStepRequest, ModelRunner, TurnError, TurnMessage};

use crate::provider_settings::ProviderSettingsService;

pub struct ProviderModelRunner {
    client: ProviderClient,
    model: String,
    tools: Vec<ProviderTool>,
}

impl ProviderModelRunner {
    pub fn from_provider_settings(
        settings: Option<&ProviderSettingsService>,
        surface: &AgentToolSurface,
    ) -> Result<Self, TurnError> {
        if let Some(settings) = settings {
            if let Some(runtime) = settings
                .load_default_runtime_config()
                .map_err(map_settings_error)?
            {
                return Self::from_runtime_config(
                    runtime.provider_kind,
                    runtime.model,
                    runtime.base_url,
                    runtime.api_key,
                    surface,
                );
            }
        }

        Self::from_env(surface)
    }

    pub fn from_env(surface: &AgentToolSurface) -> Result<Self, TurnError> {
        let model = required_env("ARGUSX_MODEL")?;
        let dialect = optional_env("ARGUSX_PROVIDER_DIALECT")
            .as_deref()
            .map(parse_dialect)
            .transpose()?
            .unwrap_or(Dialect::Openai);
        let replay_file = optional_env("ARGUSX_PROVIDER_REPLAY_FILE");

        let config = match replay_file {
            Some(path) => ProviderConfig::new(dialect, "http://unused", "test-key")
                .with_dev_options(ProviderDevOptions::replay(path, ReplayTiming::Fast)),
            None => ProviderConfig::new(
                dialect,
                required_env("ARGUSX_PROVIDER_BASE_URL")?,
                required_env("ARGUSX_PROVIDER_API_KEY")?,
            ),
        };

        Self::new(model, config, surface)
    }

    pub fn from_runtime_config(
        provider_kind: crate::provider_settings::ProviderKind,
        model: String,
        base_url: String,
        api_key: String,
        surface: &AgentToolSurface,
    ) -> Result<Self, TurnError> {
        Self::new(
            model,
            ProviderConfig::new(provider_kind.dialect(), base_url, api_key),
            surface,
        )
    }

    pub fn from_replay(
        model: &str,
        path: PathBuf,
        surface: &AgentToolSurface,
    ) -> Result<Self, TurnError> {
        Self::new(
            model.to_string(),
            ProviderConfig::new(Dialect::Openai, "http://unused", "test-key")
                .with_dev_options(ProviderDevOptions::replay(path, ReplayTiming::Fast)),
            surface,
        )
    }

    fn new(model: String, config: ProviderConfig, surface: &AgentToolSurface) -> Result<Self, TurnError> {
        Ok(Self {
            client: ProviderClient::new(config).map_err(map_provider_error)?,
            model,
            tools: tool_definitions(surface)?,
        })
    }

    fn build_request(&self, request: &LlmStepRequest) -> Request {
        ChatCompletionsOptions {
            model: self.model.clone(),
            messages: map_messages(&request.messages, request.system_prompt.as_deref()),
            stream: Some(true),
            tools: request.allow_tools.then(|| self.tools.clone()),
            tool_choice: request
                .allow_tools
                .then(|| ToolChoice::String("auto".to_string())),
            parallel_tool_calls: request.allow_tools.then_some(true),
            ..Default::default()
        }
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tools
            .iter()
            .map(|tool| tool.function.name.clone())
            .collect()
    }
}

#[async_trait]
impl ModelRunner for ProviderModelRunner {
    async fn start(&self, request: LlmStepRequest) -> Result<argus_core::ResponseStream, TurnError> {
        self.client
            .stream(self.build_request(&request))
            .map_err(map_provider_error)
    }
}

fn map_messages(
    messages: &[std::sync::Arc<TurnMessage>],
    system_prompt: Option<&str>,
) -> Vec<ChatMessage> {
    let mut mapped = Vec::with_capacity(messages.len() + usize::from(system_prompt.is_some()));
    if let Some(prompt) = system_prompt {
        mapped.push(ChatMessage {
            role: Role::System,
            content: Some(prompt.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            extra: Map::default(),
        });
    }

    mapped.extend(messages.iter().map(|message| map_message(message.as_ref())));
    mapped
}

fn map_message(message: &TurnMessage) -> ChatMessage {
    match message {
        TurnMessage::User { content } => ChatMessage {
            role: Role::User,
            content: Some(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            extra: Map::default(),
        },
        TurnMessage::AssistantText { content } => ChatMessage {
            role: Role::Assistant,
            content: Some(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            extra: Map::default(),
        },
        TurnMessage::AssistantToolCalls { content, calls } => ChatMessage {
            role: Role::Assistant,
            content: content.as_ref().map(ToString::to_string),
            name: None,
            tool_calls: Some(calls.iter().map(|call| map_tool_call(call.as_ref())).collect()),
            tool_call_id: None,
            extra: Map::default(),
        },
        TurnMessage::ToolResult {
            call_id,
            tool_name,
            content,
            ..
        } => ChatMessage {
            role: Role::Tool,
            content: Some(content.to_string()),
            name: Some(tool_name.to_string()),
            tool_calls: None,
            tool_call_id: Some(call_id.to_string()),
            extra: Map::default(),
        },
        TurnMessage::SystemNote { content } => ChatMessage {
            role: Role::System,
            content: Some(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            extra: Map::default(),
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
        } => provider_tool_call(call_id, name, arguments_json),
        argus_core::ToolCall::Builtin(call) => {
            provider_tool_call(&call.call_id, call.builtin.canonical_name(), &call.arguments_json)
        }
        argus_core::ToolCall::Mcp(call) => provider_tool_call(
            &call.id,
            call.name.as_deref().unwrap_or_default(),
            call.arguments_json.as_deref().unwrap_or("{}"),
        ),
    }
}

fn provider_tool_call(call_id: &str, name: &str, arguments_json: &str) -> ProviderToolCall {
    ProviderToolCall {
        id: call_id.to_string(),
        type_: "function".to_string(),
        function: FunctionCall {
            name: name.to_string(),
            arguments: arguments_json.to_string(),
            extra: Map::default(),
        },
        extra: Map::default(),
    }
}

fn tool_definitions(surface: &AgentToolSurface) -> Result<Vec<ProviderTool>, TurnError> {
    surface
        .tool_specs_from_current_dir()
        .map_err(map_tool_init_error)?
        .into_iter()
        .map(to_provider_tool)
        .collect()
}

fn to_provider_tool(spec: ToolSpec) -> Result<ProviderTool, TurnError> {
    Ok(ProviderTool {
        type_: "function".to_string(),
        function: FunctionDefinition {
            name: spec.name,
            description: Some(spec.description),
            parameters: spec.input_schema,
            strict: None,
            extra: Map::<String, Value>::default(),
        },
        extra: Map::<String, Value>::default(),
    })
}

fn parse_dialect(raw: &str) -> Result<Dialect, TurnError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "openai" => Ok(Dialect::Openai),
        "zai" => Ok(Dialect::Zai),
        other => Err(TurnError::Runtime(format!(
            "unsupported ARGUSX_PROVIDER_DIALECT `{other}`"
        ))),
    }
}

fn required_env(name: &str) -> Result<String, TurnError> {
    optional_env(name).ok_or_else(|| TurnError::Runtime(format!("missing env var {name}")))
}

fn optional_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn map_provider_error(err: provider::Error) -> TurnError {
    TurnError::Runtime(err.to_string())
}

fn map_settings_error(err: crate::provider_settings::ProviderSettingsError) -> TurnError {
    TurnError::Runtime(err.to_string())
}

fn map_tool_init_error(err: impl std::fmt::Display) -> TurnError {
    TurnError::Runtime(err.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use argus_core::{Builtin, BuiltinToolCall, ToolCall};

    use super::*;

    #[test]
    fn build_request_maps_turn_messages() {
        let surface = agent::build_agent_tool_surface(serde_json::json!({
            "builtins": ["read", "update_plan"]
        }))
        .unwrap();
        let runner =
            ProviderModelRunner::from_replay("gpt-test", PathBuf::from("fixture.sse"), &surface)
                .unwrap();
        let request = LlmStepRequest {
            session_id: "session-1".into(),
            turn_id: "turn-1".into(),
            step_index: 1,
            messages: Arc::from([
                Arc::new(TurnMessage::User {
                    content: "read file".into(),
                }),
                Arc::new(TurnMessage::AssistantToolCalls {
                    content: None,
                    calls: Arc::from([Arc::new(ToolCall::Builtin(BuiltinToolCall {
                        sequence: 0,
                        call_id: "call-1".into(),
                        builtin: Builtin::Read,
                        arguments_json: r#"{"path":"Cargo.toml"}"#.into(),
                    }))]),
                }),
                Arc::new(TurnMessage::ToolResult {
                    call_id: "call-1".into(),
                    tool_name: "read".into(),
                    content: r#"{"content":"ok"}"#.into(),
                    is_error: false,
                }),
            ]),
            system_prompt: None,
            allow_tools: false,
        };

        let built = runner.build_request(&request);

        assert_eq!(built.model, "gpt-test");
        assert_eq!(built.messages.len(), 3);
        assert!(matches!(built.messages[0].role, Role::User));
        assert!(matches!(built.messages[1].role, Role::Assistant));
        assert_eq!(built.messages[1].tool_calls.as_ref().unwrap()[0].id, "call-1");
        assert!(matches!(built.messages[2].role, Role::Tool));
        assert_eq!(
            built.messages[2].tool_call_id.as_deref(),
            Some("call-1")
        );
    }

    #[test]
    fn build_request_includes_read_only_tools_when_allowed() {
        let surface = agent::build_agent_tool_surface(serde_json::json!({
            "builtins": ["read", "glob", "grep", "update_plan"]
        }))
        .unwrap();
        let runner =
            ProviderModelRunner::from_replay("gpt-test", PathBuf::from("fixture.sse"), &surface)
                .unwrap();
        let request = LlmStepRequest {
            session_id: "session-1".into(),
            turn_id: "turn-1".into(),
            step_index: 0,
            messages: Arc::from([Arc::new(TurnMessage::User {
                content: "find toml".into(),
            })]),
            system_prompt: None,
            allow_tools: true,
        };

        let built = runner.build_request(&request);
        let tools = built.tools.expect("tools should be present");

        assert_eq!(tools.len(), 4);
        assert_eq!(tools[0].function.name, "read");
        assert_eq!(tools[1].function.name, "glob");
        assert_eq!(tools[2].function.name, "grep");
        assert_eq!(tools[3].function.name, "update_plan");
        assert!(matches!(
            built.tool_choice,
            Some(ToolChoice::String(ref choice)) if choice == "auto"
        ));
        assert_eq!(built.parallel_tool_calls, Some(true));
    }

    #[test]
    fn build_request_includes_update_plan_tool_when_allowed() {
        let surface = agent::build_agent_tool_surface(serde_json::json!({
            "builtins": ["read", "update_plan"]
        }))
        .unwrap();
        let runner =
            ProviderModelRunner::from_replay("gpt-test", PathBuf::from("fixture.sse"), &surface)
                .unwrap();
        let request = LlmStepRequest {
            session_id: "session-1".into(),
            turn_id: "turn-1".into(),
            step_index: 0,
            messages: Arc::from([Arc::new(TurnMessage::User {
                content: "keep a plan".into(),
            })]),
            system_prompt: None,
            allow_tools: true,
        };

        let built = runner.build_request(&request);
        let tools = built.tools.expect("tools should be present");

        assert!(tools.iter().any(|tool| tool.function.name == "update_plan"));
    }

    #[test]
    fn build_request_prepends_system_prompt() {
        let surface = agent::build_agent_tool_surface(serde_json::json!({
            "builtins": ["read"]
        }))
        .unwrap();
        let runner =
            ProviderModelRunner::from_replay("gpt-test", PathBuf::from("fixture.sse"), &surface)
                .unwrap();
        let request = LlmStepRequest {
            session_id: "session-1".into(),
            turn_id: "turn-1".into(),
            step_index: 0,
            messages: Arc::from([Arc::new(TurnMessage::User {
                content: "hello".into(),
            })]),
            system_prompt: Some("You are a planner.".into()),
            allow_tools: false,
        };

        let built = runner.build_request(&request);

        assert_eq!(built.messages.len(), 2);
        assert!(matches!(built.messages[0].role, Role::System));
        assert_eq!(built.messages[0].content.as_deref(), Some("You are a planner."));
        assert!(matches!(built.messages[1].role, Role::User));
        assert_eq!(request.messages.len(), 1);
    }
}
