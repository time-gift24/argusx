use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::str::FromStr;

use crate::error::{PromptLabError, Result};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TargetLevel {
    Step,
    Sop,
}

impl TargetLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Step => "step",
            Self::Sop => "sop",
        }
    }
}

impl fmt::Display for TargetLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TargetLevel {
    type Err = PromptLabError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "step" => Ok(Self::Step),
            "sop" => Ok(Self::Sop),
            _ => Err(PromptLabError::InvalidEnum {
                field: "target_level",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChecklistStatus {
    Active,
    Inactive,
    Draft,
}

impl ChecklistStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Draft => "draft",
        }
    }
}

impl fmt::Display for ChecklistStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ChecklistStatus {
    type Err = PromptLabError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "draft" => Ok(Self::Draft),
            _ => Err(PromptLabError::InvalidEnum {
                field: "status",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceType {
    Ai,
    Manual,
}

impl SourceType {
    pub fn as_i64(self) -> i64 {
        match self {
            Self::Ai => 1,
            Self::Manual => 2,
        }
    }

    pub fn from_i64(value: i64) -> Result<Self> {
        match value {
            1 => Ok(Self::Ai),
            2 => Ok(Self::Manual),
            _ => Err(PromptLabError::InvalidEnum {
                field: "source_type",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecStatus {
    Pending,
    Success,
    ApiError,
    ParseFailed,
}

impl ExecStatus {
    pub fn as_i64(self) -> i64 {
        match self {
            Self::Pending => 0,
            Self::Success => 1,
            Self::ApiError => 2,
            Self::ParseFailed => 3,
        }
    }

    pub fn from_i64(value: i64) -> Result<Self> {
        match value {
            0 => Ok(Self::Pending),
            1 => Ok(Self::Success),
            2 => Ok(Self::ApiError),
            3 => Ok(Self::ParseFailed),
            _ => Err(PromptLabError::InvalidEnum {
                field: "exec_status",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChecklistItem {
    pub id: i64,
    pub name: String,
    pub prompt: String,
    pub target_level: TargetLevel,
    pub result_schema: Option<Value>,
    pub version: i64,
    pub status: ChecklistStatus,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateChecklistItemInput {
    pub name: String,
    pub prompt: String,
    pub target_level: TargetLevel,
    pub result_schema: Option<Value>,
    pub version: Option<i64>,
    pub status: ChecklistStatus,
    pub created_by: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct UpdateChecklistItemInput {
    pub id: i64,
    pub name: Option<String>,
    pub prompt: Option<String>,
    pub target_level: Option<TargetLevel>,
    pub result_schema: Option<Value>,
    pub version: Option<i64>,
    pub status: Option<ChecklistStatus>,
    pub updated_by: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ChecklistFilter {
    pub status: Option<ChecklistStatus>,
    pub target_level: Option<TargetLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoldenSetItem {
    pub golden_set_id: i64,
    pub checklist_item_id: i64,
    pub sort_order: i64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct BindGoldenSetItemInput {
    pub golden_set_id: i64,
    pub checklist_item_id: i64,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckResult {
    pub id: i64,
    pub context_type: String,
    pub context_id: i64,
    pub check_item_id: i64,
    pub source_type: SourceType,
    pub operator_id: Option<String>,
    pub result: Option<Value>,
    pub is_pass: bool,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct UpsertCheckResultInput {
    pub id: Option<i64>,
    pub context_type: String,
    pub context_id: i64,
    pub check_item_id: i64,
    pub source_type: SourceType,
    pub operator_id: Option<String>,
    pub result: Option<Value>,
    pub is_pass: bool,
}

#[derive(Debug, Clone)]
pub struct CheckResultFilter {
    pub context_type: Option<String>,
    pub context_id: Option<i64>,
    pub check_item_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiExecutionLog {
    pub id: i64,
    pub check_result_id: Option<i64>,
    pub context_type: String,
    pub context_id: i64,
    pub check_item_id: i64,
    pub model_provider: Option<String>,
    pub model_version: String,
    pub temperature: Option<f64>,
    pub prompt_snapshot: Option<String>,
    pub raw_output: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub exec_status: ExecStatus,
    pub error_message: Option<String>,
    pub latency_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AppendAiExecutionLogInput {
    pub check_result_id: Option<i64>,
    pub context_type: String,
    pub context_id: i64,
    pub check_item_id: i64,
    pub model_provider: Option<String>,
    pub model_version: String,
    pub temperature: Option<f64>,
    pub prompt_snapshot: Option<String>,
    pub raw_output: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub exec_status: ExecStatus,
    pub error_message: Option<String>,
    pub latency_ms: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct AiExecutionLogFilter {
    pub context_type: Option<String>,
    pub context_id: Option<i64>,
    pub check_item_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SopStatus {
    Active,
    Inactive,
    Draft,
}

impl SopStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Draft => "draft",
        }
    }
}

impl fmt::Display for SopStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SopStatus {
    type Err = PromptLabError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "draft" => Ok(Self::Draft),
            _ => Err(PromptLabError::InvalidEnum {
                field: "status",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sop {
    pub id: i64,
    pub sop_id: String,
    pub name: String,
    pub ticket_id: Option<String>,
    pub version: i64,
    pub detect: Option<Value>,
    pub handle: Option<Value>,
    pub verification: Option<Value>,
    pub rollback: Option<Value>,
    pub status: SopStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CreateSopInput {
    pub sop_id: String,
    pub name: String,
    pub ticket_id: Option<String>,
    pub version: Option<i64>,
    pub detect: Option<Value>,
    pub handle: Option<Value>,
    pub verification: Option<Value>,
    pub rollback: Option<Value>,
    pub status: SopStatus,
}

#[derive(Debug, Clone)]
pub struct UpdateSopInput {
    pub id: i64,
    pub sop_id: Option<String>,
    pub name: Option<String>,
    pub ticket_id: Option<String>,
    pub version: Option<i64>,
    pub detect: Option<Value>,
    pub handle: Option<Value>,
    pub verification: Option<Value>,
    pub rollback: Option<Value>,
    pub status: Option<SopStatus>,
}

#[derive(Debug, Clone)]
pub struct SopFilter {
    pub status: Option<SopStatus>,
    pub ticket_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SopStep {
    pub id: i64,
    pub sop_id: String,
    pub name: String,
    pub version: i64,
    pub operation: Option<Value>,
    pub verification: Option<Value>,
    pub impact_analysis: Option<Value>,
    pub rollback: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CreateSopStepInput {
    pub sop_id: String,
    pub name: String,
    pub version: Option<i64>,
    pub operation: Option<Value>,
    pub verification: Option<Value>,
    pub impact_analysis: Option<Value>,
    pub rollback: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct UpdateSopStepInput {
    pub id: i64,
    pub name: Option<String>,
    pub version: Option<i64>,
    pub operation: Option<Value>,
    pub verification: Option<Value>,
    pub impact_analysis: Option<Value>,
    pub rollback: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct SopStepFilter {
    pub sop_id: Option<String>,
}
