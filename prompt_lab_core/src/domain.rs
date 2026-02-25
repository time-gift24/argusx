use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::str::FromStr;

use crate::error::{PromptLabError, Result};

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
#[serde(rename_all = "snake_case")]
pub enum ChecklistContextType {
    Sop,
    SopProcedureDetect,
    SopProcedureHandle,
    SopProcedureVerification,
    SopProcedureRollback,
    SopStepOperation,
    SopStepVerification,
    SopStepImpactAnalysis,
    SopStepRollback,
    SopStepCommon,
}

impl ChecklistContextType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sop => "sop",
            Self::SopProcedureDetect => "sop_procedure_detect",
            Self::SopProcedureHandle => "sop_procedure_handle",
            Self::SopProcedureVerification => "sop_procedure_verification",
            Self::SopProcedureRollback => "sop_procedure_rollback",
            Self::SopStepOperation => "sop_step_operation",
            Self::SopStepVerification => "sop_step_verification",
            Self::SopStepImpactAnalysis => "sop_step_impact_analysis",
            Self::SopStepRollback => "sop_step_rollback",
            Self::SopStepCommon => "sop_step_common",
        }
    }
}

impl fmt::Display for ChecklistContextType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ChecklistContextType {
    type Err = PromptLabError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "sop" => Ok(Self::Sop),
            "sop_procedure_detect" => Ok(Self::SopProcedureDetect),
            "sop_procedure_handle" => Ok(Self::SopProcedureHandle),
            "sop_procedure_verification" => Ok(Self::SopProcedureVerification),
            "sop_procedure_rollback" => Ok(Self::SopProcedureRollback),
            "sop_step_operation" => Ok(Self::SopStepOperation),
            "sop_step_verification" => Ok(Self::SopStepVerification),
            "sop_step_impact_analysis" => Ok(Self::SopStepImpactAnalysis),
            "sop_step_rollback" => Ok(Self::SopStepRollback),
            "sop_step_common" => Ok(Self::SopStepCommon),
            _ => Err(PromptLabError::InvalidEnum {
                field: "context_type",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
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
    pub context_type: ChecklistContextType,
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
    pub context_type: ChecklistContextType,
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
    pub context_type: Option<ChecklistContextType>,
    pub result_schema: Option<Value>,
    pub version: Option<i64>,
    pub status: Option<ChecklistStatus>,
    pub updated_by: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ChecklistFilter {
    pub status: Option<ChecklistStatus>,
    pub context_type: Option<ChecklistContextType>,
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
    pub context_key: String,
    pub check_item_id: Option<i64>,
    pub source_type: SourceType,
    pub operator_id: Option<String>,
    pub result: Option<Value>,
    pub is_pass: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct UpsertCheckResultInput {
    pub id: Option<i64>,
    pub context_type: String,
    pub context_key: String,
    pub check_item_id: Option<i64>,
    pub source_type: SourceType,
    pub operator_id: Option<String>,
    pub result: Option<Value>,
    pub is_pass: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct CheckResultFilter {
    pub context_type: Option<String>,
    pub context_key: Option<String>,
    pub check_item_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiExecutionLog {
    pub id: i64,
    pub check_result_id: Option<i64>,
    pub context_type: String,
    pub context_key: String,
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
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct AppendAiExecutionLogInput {
    pub check_result_id: Option<i64>,
    pub context_type: String,
    pub context_key: String,
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
    pub context_key: Option<String>,
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
    pub detect: Vec<SopStage>,
    pub handle: Vec<SopStage>,
    pub verification: Vec<SopStage>,
    pub rollback: Vec<SopStage>,
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
    pub detect: Vec<SopStage>,
    pub handle: Vec<SopStage>,
    pub verification: Vec<SopStage>,
    pub rollback: Vec<SopStage>,
    pub status: SopStatus,
}

#[derive(Debug, Clone)]
pub struct UpdateSopInput {
    pub id: i64,
    pub sop_id: Option<String>,
    pub name: Option<String>,
    pub ticket_id: Option<String>,
    pub version: Option<i64>,
    pub detect: Vec<SopStage>,
    pub handle: Vec<SopStage>,
    pub verification: Vec<SopStage>,
    pub rollback: Vec<SopStage>,
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
    pub operation: Option<String>,
    pub verification: Option<String>,
    pub impact_analysis: Option<String>,
    pub rollback: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SopStepRef {
    pub sop_step_id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SopStage {
    pub name: String,
    pub steps: Vec<SopStepRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SopAggregate {
    pub sop: Sop,
    pub detect_steps: Vec<SopStep>,
    pub handle_steps: Vec<SopStep>,
    pub verification_steps: Vec<SopStep>,
    pub rollback_steps: Vec<SopStep>,
}

#[derive(Debug, Clone)]
pub struct CreateSopStepInput {
    pub sop_id: String,
    pub name: String,
    pub version: Option<i64>,
    pub operation: Option<String>,
    pub verification: Option<String>,
    pub impact_analysis: Option<String>,
    pub rollback: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateSopStepInput {
    pub id: i64,
    pub name: Option<String>,
    pub version: Option<i64>,
    pub operation: Option<String>,
    pub verification: Option<String>,
    pub impact_analysis: Option<String>,
    pub rollback: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SopStepFilter {
    pub sop_id: Option<String>,
}
