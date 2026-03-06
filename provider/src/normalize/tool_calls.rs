use argus_core::ZaiMcpType;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedZaiMcpPayload {
    pub mcp_type: ZaiMcpType,
    pub server_label: Option<String>,
    pub name: Option<String>,
    pub arguments_json: Option<String>,
    pub output_json: Option<String>,
    pub tools_json: Option<String>,
    pub error: Option<String>,
}

pub fn is_mcp_call(call_type: Option<&str>, name: Option<&str>) -> bool {
    matches!(call_type, Some("mcp")) || name.is_some_and(|n| n.starts_with("__mcp__"))
}

pub fn parse_zai_mcp_json(
    raw: &str,
    fallback_name: Option<&str>,
) -> Result<ParsedZaiMcpPayload, String> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|err| format!("invalid mcp json: {err}; payload={raw}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "mcp json payload must be an object".to_string())?;

    let mcp_type = match object.get("type").and_then(Value::as_str) {
        Some("mcp_call") => ZaiMcpType::McpCall,
        Some("mcp_list_tools") => ZaiMcpType::McpListTools,
        Some(other) => ZaiMcpType::Unknown(other.to_string()),
        None => ZaiMcpType::Unknown("unknown".to_string()),
    };

    let server_label = object
        .get("server_label")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| fallback_name.map(ToString::to_string));

    let arguments_json = object.get("arguments").map(value_to_json_string);
    let output_json = object.get("output").map(value_to_json_string);
    let tools_json = object.get("tools").map(value_to_json_string);
    let error = object.get("error").map(value_to_plain_string);

    Ok(ParsedZaiMcpPayload {
        mcp_type,
        server_label,
        name,
        arguments_json,
        output_json,
        tools_json,
        error,
    })
}

fn value_to_json_string(value: &Value) -> String {
    if let Some(raw) = value.as_str() {
        raw.to_string()
    } else {
        value.to_string()
    }
}

fn value_to_plain_string(value: &Value) -> String {
    value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| value.to_string())
}
