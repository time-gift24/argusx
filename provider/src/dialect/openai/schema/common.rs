use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

pub(crate) fn map_is_empty(map: &Map<String, Value>) -> bool {
    map.is_empty()
}

macro_rules! string_enum_with_unknown {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $name {
            $($variant,)+
            Unknown(String),
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                match self {
                    $(Self::$variant => serializer.serialize_str($value),)+
                    Self::Unknown(value) => serializer.serialize_str(value),
                }
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                match raw.as_str() {
                    $($value => Ok(Self::$variant),)+
                    _ => Ok(Self::Unknown(raw)),
                }
            }
        }
    };
}

string_enum_with_unknown!(Role {
    System => "system",
    User => "user",
    Assistant => "assistant",
    Tool => "tool",
    Developer => "developer"
});

string_enum_with_unknown!(ReasoningEffort {
    Low => "low",
    Medium => "medium",
    High => "high"
});

string_enum_with_unknown!(Verbosity {
    Low => "low",
    Medium => "medium",
    High => "high"
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tool {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: FunctionDefinition,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub function: FunctionCall,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolChoice {
    String(String),
    Specific(ToolChoiceSpecific),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolChoiceSpecific {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: FunctionChoice,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionChoice {
    pub name: String,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum StopSequences {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_object", alias = "jsonObject", alias = "json-object")]
    JsonObject,
    #[serde(rename = "json_schema", alias = "jsonSchema", alias = "json-schema")]
    JsonSchema { json_schema: JsonSchemaDefinition },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonSchemaDefinition {
    pub name: String,
    pub schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogProbs {
    pub content: Vec<ContentLogProb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<Vec<ContentLogProb>>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentLogProb {
    pub token: String,
    pub logprob: f64,
    pub bytes: Option<Vec<u8>>,
    pub top_logprobs: Vec<TopLogProb>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopLogProb {
    pub token: String,
    pub logprob: f64,
    pub bytes: Option<Vec<u8>>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamErrorStructured {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    #[serde(default, flatten, skip_serializing_if = "map_is_empty")]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum StreamError {
    Structured(StreamErrorStructured),
    Raw(String),
}

impl StreamError {
    pub fn message(&self) -> String {
        match self {
            Self::Structured(v) => v
                .message
                .clone()
                .or_else(|| {
                    v.extra
                        .get("message")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                })
                .unwrap_or_else(|| "unknown stream error".to_string()),
            Self::Raw(v) => v.clone(),
        }
    }
}
