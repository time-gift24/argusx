use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    #[serde(rename = "openai_compatible")]
    OpenAiCompatible,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiCompatible => "openai_compatible",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "openai_compatible" => Some(Self::OpenAiCompatible),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfileSummary {
    pub id: String,
    pub provider_kind: ProviderKind,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProviderProfileInput {
    pub id: Option<String>,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestProviderProfileInput {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConnectionResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRuntimeConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderProfileRecord {
    pub id: String,
    pub provider_kind: ProviderKind,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_key_ciphertext: Vec<u8>,
    pub api_key_nonce: Vec<u8>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl ProviderProfileRecord {
    pub fn summary(&self) -> ProviderProfileSummary {
        ProviderProfileSummary {
            id: self.id.clone(),
            provider_kind: self.provider_kind,
            name: self.name.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            is_default: self.is_default,
        }
    }
}
