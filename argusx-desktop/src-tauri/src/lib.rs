use argusx_common::config::{DatabaseConfig, Settings};
use prompt_lab_core::PromptLab;
use prompt_lab_core::{
    AiExecutionLog, CheckResult as CheckResultModel, ChecklistItem, ChecklistStatus, ExecStatus,
    GoldenSetItem, SourceType, TargetLevel, CreateSopInput, UpdateSopInput, SopFilter, Sop,
    SopAggregate, SopStep, CreateSopStepInput, UpdateSopStepInput, SopStepFilter,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tauri::{Manager, State};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl From<prompt_lab_core::PromptLabError> for ApiError {
    fn from(e: prompt_lab_core::PromptLabError) -> Self {
        ApiError {
            code: e.code().as_str().to_string(),
            message: e.to_string(),
        }
    }
}

// ============================================================================
// Wrapper Types for Tauri Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetLevelInput {
    Step,
    Sop,
}

impl From<TargetLevelInput> for TargetLevel {
    fn from(v: TargetLevelInput) -> Self {
        match v {
            TargetLevelInput::Step => TargetLevel::Step,
            TargetLevelInput::Sop => TargetLevel::Sop,
        }
    }
}

impl From<TargetLevel> for TargetLevelInput {
    fn from(v: TargetLevel) -> Self {
        match v {
            TargetLevel::Step => TargetLevelInput::Step,
            TargetLevel::Sop => TargetLevelInput::Sop,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChecklistStatusInput {
    Active,
    Inactive,
    Draft,
}

impl From<ChecklistStatusInput> for ChecklistStatus {
    fn from(v: ChecklistStatusInput) -> Self {
        match v {
            ChecklistStatusInput::Active => ChecklistStatus::Active,
            ChecklistStatusInput::Inactive => ChecklistStatus::Inactive,
            ChecklistStatusInput::Draft => ChecklistStatus::Draft,
        }
    }
}

impl From<ChecklistStatus> for ChecklistStatusInput {
    fn from(v: ChecklistStatus) -> Self {
        match v {
            ChecklistStatus::Active => ChecklistStatusInput::Active,
            ChecklistStatus::Inactive => ChecklistStatusInput::Inactive,
            ChecklistStatus::Draft => ChecklistStatusInput::Draft,
        }
    }
}

// Checklist Types
#[derive(Debug, Deserialize)]
pub struct CreateChecklistItemInput {
    pub name: String,
    pub prompt: String,
    pub target_level: TargetLevelInput,
    pub result_schema: Option<Value>,
    pub version: Option<i64>,
    pub status: ChecklistStatusInput,
    pub created_by: Option<i64>,
}

impl From<CreateChecklistItemInput> for prompt_lab_core::CreateChecklistItemInput {
    fn from(v: CreateChecklistItemInput) -> Self {
        prompt_lab_core::CreateChecklistItemInput {
            name: v.name,
            prompt: v.prompt,
            target_level: v.target_level.into(),
            result_schema: v.result_schema,
            version: v.version,
            status: v.status.into(),
            created_by: v.created_by,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateChecklistItemInput {
    pub id: i64,
    pub name: Option<String>,
    pub prompt: Option<String>,
    pub target_level: Option<TargetLevelInput>,
    pub result_schema: Option<Value>,
    pub version: Option<i64>,
    pub status: Option<ChecklistStatusInput>,
    pub updated_by: Option<i64>,
}

impl From<UpdateChecklistItemInput> for prompt_lab_core::UpdateChecklistItemInput {
    fn from(v: UpdateChecklistItemInput) -> Self {
        prompt_lab_core::UpdateChecklistItemInput {
            id: v.id,
            name: v.name,
            prompt: v.prompt,
            target_level: v.target_level.map(|t| t.into()),
            result_schema: v.result_schema,
            version: v.version,
            status: v.status.map(|s| s.into()),
            updated_by: v.updated_by,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ChecklistFilter {
    pub status: Option<ChecklistStatusInput>,
    pub target_level: Option<TargetLevelInput>,
}

impl From<ChecklistFilter> for prompt_lab_core::ChecklistFilter {
    fn from(v: ChecklistFilter) -> Self {
        prompt_lab_core::ChecklistFilter {
            status: v.status.map(|s| s.into()),
            target_level: v.target_level.map(|t| t.into()),
        }
    }
}

// GoldenSet Types
#[derive(Debug, Deserialize)]
pub struct BindGoldenSetItemInput {
    pub golden_set_id: i64,
    pub checklist_item_id: i64,
    pub sort_order: i64,
}

impl From<BindGoldenSetItemInput> for prompt_lab_core::BindGoldenSetItemInput {
    fn from(v: BindGoldenSetItemInput) -> Self {
        prompt_lab_core::BindGoldenSetItemInput {
            golden_set_id: v.golden_set_id,
            checklist_item_id: v.checklist_item_id,
            sort_order: v.sort_order,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UnbindGoldenSetItemInput {
    pub golden_set_id: i64,
    pub checklist_item_id: i64,
}

// CheckResult Types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceTypeInput {
    Ai,
    Manual,
}

impl From<SourceTypeInput> for SourceType {
    fn from(v: SourceTypeInput) -> Self {
        match v {
            SourceTypeInput::Ai => SourceType::Ai,
            SourceTypeInput::Manual => SourceType::Manual,
        }
    }
}

impl From<SourceType> for SourceTypeInput {
    fn from(v: SourceType) -> Self {
        match v {
            SourceType::Ai => SourceTypeInput::Ai,
            SourceType::Manual => SourceTypeInput::Manual,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertCheckResultInput {
    pub id: Option<i64>,
    pub context_type: String,
    pub context_key: String,
    pub check_item_id: Option<i64>,
    pub source_type: SourceTypeInput,
    pub operator_id: Option<String>,
    pub result: Option<Value>,
    pub is_pass: Option<bool>,
}

impl From<UpsertCheckResultInput> for prompt_lab_core::UpsertCheckResultInput {
    fn from(v: UpsertCheckResultInput) -> Self {
        prompt_lab_core::UpsertCheckResultInput {
            id: v.id,
            context_type: v.context_type,
            context_key: v.context_key,
            check_item_id: v.check_item_id,
            source_type: v.source_type.into(),
            operator_id: v.operator_id,
            result: v.result,
            is_pass: v.is_pass,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct CheckResultFilter {
    pub context_type: Option<String>,
    pub context_key: Option<String>,
    pub check_item_id: Option<i64>,
}

impl From<CheckResultFilter> for prompt_lab_core::CheckResultFilter {
    fn from(v: CheckResultFilter) -> Self {
        prompt_lab_core::CheckResultFilter {
            context_type: v.context_type,
            context_key: v.context_key,
            check_item_id: v.check_item_id,
        }
    }
}

// AiExecutionLog Types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecStatusInput {
    Pending,
    Success,
    ApiError,
    ParseFailed,
}

impl From<ExecStatusInput> for ExecStatus {
    fn from(v: ExecStatusInput) -> Self {
        match v {
            ExecStatusInput::Pending => ExecStatus::Pending,
            ExecStatusInput::Success => ExecStatus::Success,
            ExecStatusInput::ApiError => ExecStatus::ApiError,
            ExecStatusInput::ParseFailed => ExecStatus::ParseFailed,
        }
    }
}

impl From<ExecStatus> for ExecStatusInput {
    fn from(v: ExecStatus) -> Self {
        match v {
            ExecStatus::Pending => ExecStatusInput::Pending,
            ExecStatus::Success => ExecStatusInput::Success,
            ExecStatus::ApiError => ExecStatusInput::ApiError,
            ExecStatus::ParseFailed => ExecStatusInput::ParseFailed,
        }
    }
}

#[derive(Debug, Deserialize)]
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
    pub exec_status: ExecStatusInput,
    pub error_message: Option<String>,
    pub latency_ms: Option<i64>,
}

impl From<AppendAiExecutionLogInput> for prompt_lab_core::AppendAiExecutionLogInput {
    fn from(v: AppendAiExecutionLogInput) -> Self {
        prompt_lab_core::AppendAiExecutionLogInput {
            check_result_id: v.check_result_id,
            context_type: v.context_type,
            context_key: v.context_key,
            check_item_id: v.check_item_id,
            model_provider: v.model_provider,
            model_version: v.model_version,
            temperature: v.temperature,
            prompt_snapshot: v.prompt_snapshot,
            raw_output: v.raw_output,
            input_tokens: v.input_tokens,
            output_tokens: v.output_tokens,
            exec_status: v.exec_status.into(),
            error_message: v.error_message,
            latency_ms: v.latency_ms,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct AiExecutionLogFilter {
    pub context_type: Option<String>,
    pub context_key: Option<String>,
    pub check_item_id: Option<i64>,
}

impl From<AiExecutionLogFilter> for prompt_lab_core::AiExecutionLogFilter {
    fn from(v: AiExecutionLogFilter) -> Self {
        prompt_lab_core::AiExecutionLogFilter {
            context_type: v.context_type,
            context_key: v.context_key,
            check_item_id: v.check_item_id,
        }
    }
}

// SopStatus Types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SopStatusInput {
    Active,
    Inactive,
    Draft,
}

impl From<SopStatusInput> for prompt_lab_core::SopStatus {
    fn from(v: SopStatusInput) -> Self {
        match v {
            SopStatusInput::Active => prompt_lab_core::SopStatus::Active,
            SopStatusInput::Inactive => prompt_lab_core::SopStatus::Inactive,
            SopStatusInput::Draft => prompt_lab_core::SopStatus::Draft,
        }
    }
}

impl From<prompt_lab_core::SopStatus> for SopStatusInput {
    fn from(v: prompt_lab_core::SopStatus) -> Self {
        match v {
            prompt_lab_core::SopStatus::Active => SopStatusInput::Active,
            prompt_lab_core::SopStatus::Inactive => SopStatusInput::Inactive,
            prompt_lab_core::SopStatus::Draft => SopStatusInput::Draft,
        }
    }
}

// SOP Types
#[derive(Debug, Deserialize)]
pub struct CreateSopInputInternal {
    pub sop_id: String,
    pub name: String,
    pub ticket_id: Option<String>,
    pub version: Option<i64>,
    pub detect: Option<Value>,
    pub handle: Option<Value>,
    pub verification: Option<Value>,
    pub rollback: Option<Value>,
    pub status: SopStatusInput,
}

impl From<CreateSopInputInternal> for CreateSopInput {
    fn from(v: CreateSopInputInternal) -> Self {
        CreateSopInput {
            sop_id: v.sop_id,
            name: v.name,
            ticket_id: v.ticket_id,
            version: v.version,
            detect: v.detect,
            handle: v.handle,
            verification: v.verification,
            rollback: v.rollback,
            status: v.status.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateSopInputInternal {
    pub id: i64,
    pub sop_id: Option<String>,
    pub name: Option<String>,
    pub ticket_id: Option<String>,
    pub version: Option<i64>,
    pub detect: Option<Value>,
    pub handle: Option<Value>,
    pub verification: Option<Value>,
    pub rollback: Option<Value>,
    pub status: Option<SopStatusInput>,
}

impl From<UpdateSopInputInternal> for UpdateSopInput {
    fn from(v: UpdateSopInputInternal) -> Self {
        UpdateSopInput {
            id: v.id,
            sop_id: v.sop_id,
            name: v.name,
            ticket_id: v.ticket_id,
            version: v.version,
            detect: v.detect,
            handle: v.handle,
            verification: v.verification,
            rollback: v.rollback,
            status: v.status.map(|s| s.into()),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct SopFilterInternal {
    pub status: Option<SopStatusInput>,
    pub ticket_id: Option<String>,
}

impl From<SopFilterInternal> for SopFilter {
    fn from(v: SopFilterInternal) -> Self {
        SopFilter {
            status: v.status.map(|s| s.into()),
            ticket_id: v.ticket_id,
        }
    }
}

// SopStep Types
#[derive(Debug, Deserialize)]
pub struct CreateSopStepInputInternal {
    pub sop_id: String,
    pub name: String,
    pub version: Option<i64>,
    pub operation: Option<Value>,
    pub verification: Option<Value>,
    pub impact_analysis: Option<Value>,
    pub rollback: Option<Value>,
}

impl From<CreateSopStepInputInternal> for CreateSopStepInput {
    fn from(v: CreateSopStepInputInternal) -> Self {
        CreateSopStepInput {
            sop_id: v.sop_id,
            name: v.name,
            version: v.version,
            operation: v.operation,
            verification: v.verification,
            impact_analysis: v.impact_analysis,
            rollback: v.rollback,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateSopStepInputInternal {
    pub id: i64,
    pub name: Option<String>,
    pub version: Option<i64>,
    pub operation: Option<Value>,
    pub verification: Option<Value>,
    pub impact_analysis: Option<Value>,
    pub rollback: Option<Value>,
}

impl From<UpdateSopStepInputInternal> for UpdateSopStepInput {
    fn from(v: UpdateSopStepInputInternal) -> Self {
        UpdateSopStepInput {
            id: v.id,
            name: v.name,
            version: v.version,
            operation: v.operation,
            verification: v.verification,
            impact_analysis: v.impact_analysis,
            rollback: v.rollback,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct SopStepFilterInternal {
    pub sop_id: Option<String>,
}

impl From<SopStepFilterInternal> for SopStepFilter {
    fn from(v: SopStepFilterInternal) -> Self {
        SopStepFilter {
            sop_id: v.sop_id,
        }
    }
}

// ============================================================================
// Chat Types
// ============================================================================

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub color: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ChecklistItemResponse {
    pub id: i64,
    pub name: String,
    pub prompt: String,
    pub target_level: TargetLevelInput,
    pub result_schema: Option<Value>,
    pub version: i64,
    pub status: ChecklistStatusInput,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
    pub deleted_at: Option<String>,
}

impl From<ChecklistItem> for ChecklistItemResponse {
    fn from(v: ChecklistItem) -> Self {
        ChecklistItemResponse {
            id: v.id,
            name: v.name,
            prompt: v.prompt,
            target_level: v.target_level.into(),
            result_schema: v.result_schema,
            version: v.version,
            status: v.status.into(),
            created_at: v.created_at,
            updated_at: v.updated_at,
            created_by: v.created_by,
            updated_by: v.updated_by,
            deleted_at: v.deleted_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GoldenSetItemResponse {
    pub golden_set_id: i64,
    pub checklist_item_id: i64,
    pub sort_order: i64,
    pub created_at: String,
}

impl From<GoldenSetItem> for GoldenSetItemResponse {
    fn from(v: GoldenSetItem) -> Self {
        GoldenSetItemResponse {
            golden_set_id: v.golden_set_id,
            checklist_item_id: v.checklist_item_id,
            sort_order: v.sort_order,
            created_at: v.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CheckResultResponse {
    pub id: i64,
    pub context_type: String,
    pub context_key: String,
    pub check_item_id: Option<i64>,
    pub source_type: SourceTypeInput,
    pub operator_id: Option<String>,
    pub result: Option<Value>,
    pub is_pass: bool,
    pub created_at: i64,
}

impl From<CheckResultModel> for CheckResultResponse {
    fn from(v: CheckResultModel) -> Self {
        CheckResultResponse {
            id: v.id,
            context_type: v.context_type,
            context_key: v.context_key,
            check_item_id: v.check_item_id,
            source_type: v.source_type.into(),
            operator_id: v.operator_id,
            result: v.result,
            is_pass: v.is_pass,
            created_at: v.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AiExecutionLogResponse {
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
    pub exec_status: ExecStatusInput,
    pub error_message: Option<String>,
    pub latency_ms: i64,
    pub created_at: i64,
}

impl From<AiExecutionLog> for AiExecutionLogResponse {
    fn from(v: AiExecutionLog) -> Self {
        AiExecutionLogResponse {
            id: v.id,
            check_result_id: v.check_result_id,
            context_type: v.context_type,
            context_key: v.context_key,
            check_item_id: v.check_item_id,
            model_provider: v.model_provider,
            model_version: v.model_version,
            temperature: v.temperature,
            prompt_snapshot: v.prompt_snapshot,
            raw_output: v.raw_output,
            input_tokens: v.input_tokens,
            output_tokens: v.output_tokens,
            exec_status: v.exec_status.into(),
            error_message: v.error_message,
            latency_ms: v.latency_ms,
            created_at: v.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SopResponse {
    pub id: i64,
    pub sop_id: String,
    pub name: String,
    pub ticket_id: Option<String>,
    pub version: i64,
    pub detect: Option<Value>,
    pub handle: Option<Value>,
    pub verification: Option<Value>,
    pub rollback: Option<Value>,
    pub status: SopStatusInput,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Sop> for SopResponse {
    fn from(v: Sop) -> Self {
        SopResponse {
            id: v.id,
            sop_id: v.sop_id,
            name: v.name,
            ticket_id: v.ticket_id,
            version: v.version,
            detect: v.detect,
            handle: v.handle,
            verification: v.verification,
            rollback: v.rollback,
            status: v.status.into(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SopStepResponse {
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

impl From<SopStep> for SopStepResponse {
    fn from(v: SopStep) -> Self {
        SopStepResponse {
            id: v.id,
            sop_id: v.sop_id,
            name: v.name,
            version: v.version,
            operation: v.operation,
            verification: v.verification,
            impact_analysis: v.impact_analysis,
            rollback: v.rollback,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SopAggregateResponse {
    pub sop: SopResponse,
    pub detect_steps: Vec<SopStepResponse>,
    pub handle_steps: Vec<SopStepResponse>,
    pub verification_steps: Vec<SopStepResponse>,
    pub rollback_steps: Vec<SopStepResponse>,
}

impl From<SopAggregate> for SopAggregateResponse {
    fn from(v: SopAggregate) -> Self {
        SopAggregateResponse {
            sop: v.sop.into(),
            detect_steps: v.detect_steps.into_iter().map(Into::into).collect(),
            handle_steps: v.handle_steps.into_iter().map(Into::into).collect(),
            verification_steps: v.verification_steps.into_iter().map(Into::into).collect(),
            rollback_steps: v.rollback_steps.into_iter().map(Into::into).collect(),
        }
    }
}

// ============================================================================
// Chat Commands
// ============================================================================

#[tauri::command]
async fn create_chat_session(title: Option<String>) -> Result<ChatSession, String> {
    let session = ChatSession {
        id: format!("s-{}", uuid::Uuid::new_v4()),
        title: title.unwrap_or_else(|| "New Chat".to_string()),
        color: "blue".to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
        updated_at: chrono::Utc::now().timestamp_millis(),
        status: "active".to_string(),
    };
    Ok(session)
}

#[tauri::command]
async fn list_chat_sessions() -> Result<Vec<ChatSession>, String> {
    // TODO: 后续从数据库加载
    Ok(vec![])
}

#[tauri::command]
async fn delete_chat_session(_id: String) -> Result<(), String> {
    // TODO: 后续实现
    Ok(())
}

// ============================================================================
// Checklist Commands
// ============================================================================

#[tauri::command]
async fn create_checklist_item(
    state: State<'_, Arc<PromptLab>>,
    input: CreateChecklistItemInput,
) -> Result<ChecklistItemResponse, ApiError> {
    let result = state
        .checklist_service()
        .create(input.into())
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

#[tauri::command]
async fn update_checklist_item(
    state: State<'_, Arc<PromptLab>>,
    input: UpdateChecklistItemInput,
) -> Result<ChecklistItemResponse, ApiError> {
    let result = state
        .checklist_service()
        .update(input.into())
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

#[tauri::command]
async fn delete_checklist_item(state: State<'_, Arc<PromptLab>>, id: i64) -> Result<(), ApiError> {
    state
        .checklist_service()
        .soft_delete(id)
        .await
        .map_err(ApiError::from)?;
    Ok(())
}

#[tauri::command]
async fn list_checklist_items(
    state: State<'_, Arc<PromptLab>>,
    filter: ChecklistFilter,
) -> Result<Vec<ChecklistItemResponse>, ApiError> {
    let results = state
        .checklist_service()
        .list(filter.into())
        .await
        .map_err(ApiError::from)?;
    Ok(results.into_iter().map(|i| i.into()).collect())
}

#[tauri::command]
async fn get_checklist_item(
    state: State<'_, Arc<PromptLab>>,
    id: i64,
) -> Result<ChecklistItemResponse, ApiError> {
    let result = state
        .checklist_service()
        .get(id)
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

// ============================================================================
// GoldenSet Commands
// ============================================================================

#[tauri::command]
async fn bind_golden_set_item(
    state: State<'_, Arc<PromptLab>>,
    input: BindGoldenSetItemInput,
) -> Result<GoldenSetItemResponse, ApiError> {
    let result = state
        .golden_set_service()
        .bind(input.into())
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

#[tauri::command]
async fn unbind_golden_set_item(
    state: State<'_, Arc<PromptLab>>,
    input: UnbindGoldenSetItemInput,
) -> Result<(), ApiError> {
    state
        .golden_set_service()
        .unbind(input.golden_set_id, input.checklist_item_id)
        .await
        .map_err(ApiError::from)?;
    Ok(())
}

#[tauri::command]
async fn list_golden_set_items(
    state: State<'_, Arc<PromptLab>>,
    golden_set_id: i64,
) -> Result<Vec<GoldenSetItemResponse>, ApiError> {
    let results = state
        .golden_set_service()
        .list(golden_set_id)
        .await
        .map_err(ApiError::from)?;
    Ok(results.into_iter().map(|i| i.into()).collect())
}

// ============================================================================
// CheckResult Commands
// ============================================================================

#[tauri::command]
async fn upsert_or_append_check_result(
    state: State<'_, Arc<PromptLab>>,
    input: UpsertCheckResultInput,
) -> Result<CheckResultResponse, ApiError> {
    let result = state
        .check_result_service()
        .upsert_or_append(input.into())
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

#[tauri::command]
async fn list_check_results(
    state: State<'_, Arc<PromptLab>>,
    filter: CheckResultFilter,
) -> Result<Vec<CheckResultResponse>, ApiError> {
    let results = state
        .check_result_service()
        .list(filter.into())
        .await
        .map_err(ApiError::from)?;
    Ok(results.into_iter().map(|i| i.into()).collect())
}

// ============================================================================
// AiLog Commands
// ============================================================================

#[tauri::command]
async fn append_ai_execution_log(
    state: State<'_, Arc<PromptLab>>,
    input: AppendAiExecutionLogInput,
) -> Result<AiExecutionLogResponse, ApiError> {
    let result = state
        .ai_log_service()
        .append(input.into())
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

#[tauri::command]
async fn list_ai_execution_logs(
    state: State<'_, Arc<PromptLab>>,
    filter: AiExecutionLogFilter,
) -> Result<Vec<AiExecutionLogResponse>, ApiError> {
    let results = state
        .ai_log_service()
        .list(filter.into())
        .await
        .map_err(ApiError::from)?;
    Ok(results.into_iter().map(|i| i.into()).collect())
}

// ============================================================================
// SOP Commands
// ============================================================================

#[tauri::command]
async fn create_sop(
    state: State<'_, Arc<PromptLab>>,
    input: CreateSopInputInternal,
) -> Result<SopResponse, ApiError> {
    let result = state
        .sop_service()
        .create_sop(input.into())
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

#[tauri::command]
async fn update_sop(
    state: State<'_, Arc<PromptLab>>,
    input: UpdateSopInputInternal,
) -> Result<SopResponse, ApiError> {
    let result = state
        .sop_service()
        .update_sop(input.into())
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

#[tauri::command]
async fn list_sops(
    state: State<'_, Arc<PromptLab>>,
    filter: SopFilterInternal,
) -> Result<Vec<SopResponse>, ApiError> {
    let results = state
        .sop_service()
        .list_sops(filter.into())
        .await
        .map_err(ApiError::from)?;
    Ok(results.into_iter().map(|i| i.into()).collect())
}

#[tauri::command]
async fn get_sop(
    state: State<'_, Arc<PromptLab>>,
    sop_id: String,
) -> Result<SopAggregateResponse, ApiError> {
    let result = state
        .sop_service()
        .get_sop_aggregate_by_sop_id(&sop_id)
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

#[tauri::command]
async fn delete_sop(
    state: State<'_, Arc<PromptLab>>,
    id: i64,
) -> Result<(), ApiError> {
    state
        .sop_service()
        .delete_sop(id)
        .await
        .map_err(ApiError::from)?;
    Ok(())
}

#[tauri::command]
async fn create_sop_step(
    state: State<'_, Arc<PromptLab>>,
    input: CreateSopStepInputInternal,
) -> Result<SopStepResponse, ApiError> {
    let result = state
        .sop_service()
        .create_sop_step(input.into())
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

#[tauri::command]
async fn update_sop_step(
    state: State<'_, Arc<PromptLab>>,
    input: UpdateSopStepInputInternal,
) -> Result<SopStepResponse, ApiError> {
    let result = state
        .sop_service()
        .update_sop_step(input.into())
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

#[tauri::command]
async fn list_sop_steps(
    state: State<'_, Arc<PromptLab>>,
    filter: SopStepFilterInternal,
) -> Result<Vec<SopStepResponse>, ApiError> {
    let results = state
        .sop_service()
        .list_sop_steps(filter.into())
        .await
        .map_err(ApiError::from)?;
    Ok(results.into_iter().map(|i| i.into()).collect())
}

#[tauri::command]
async fn get_sop_step(
    state: State<'_, Arc<PromptLab>>,
    id: i64,
) -> Result<SopStepResponse, ApiError> {
    let result = state
        .sop_service()
        .get_sop_step(id)
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}

#[tauri::command]
async fn delete_sop_step(
    state: State<'_, Arc<PromptLab>>,
    id: i64,
) -> Result<(), ApiError> {
    state
        .sop_service()
        .delete_sop_step(id)
        .await
        .map_err(ApiError::from)?;
    Ok(())
}

// ============================================================================
// App Entry Point
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Get app data directory for database
            let app_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data directory: {}", e))?;

            // Create prompt_lab directory if it doesn't exist
            let db_dir = app_dir.join("prompt_lab");
            std::fs::create_dir_all(&db_dir)
                .map_err(|e| format!("Failed to create database directory: {}", e))?;

            let db_path = db_dir.join("data.db");

            // Initialize PromptLab with app data directory
            let prompt_lab = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to create tokio runtime: {}", e))?
                .block_on(async {
                    let settings = Settings {
                        database: DatabaseConfig {
                            path: db_path.to_string_lossy().to_string(),
                            busy_timeout_ms: 5_000,
                            max_connections: 5,
                        },
                        logging: argusx_common::config::LoggingConfig::default(),
                    };
                    PromptLab::new(settings)
                        .await
                        .map_err(|e| format!("Failed to initialize PromptLab: {}", e))
                })?;

            app.manage(Arc::new(prompt_lab));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_checklist_item,
            update_checklist_item,
            delete_checklist_item,
            list_checklist_items,
            get_checklist_item,
            bind_golden_set_item,
            unbind_golden_set_item,
            list_golden_set_items,
            upsert_or_append_check_result,
            list_check_results,
            append_ai_execution_log,
            list_ai_execution_logs,
            create_sop,
            update_sop,
            list_sops,
            get_sop,
            delete_sop,
            create_sop_step,
            update_sop_step,
            list_sop_steps,
            get_sop_step,
            delete_sop_step,
            create_chat_session,
            list_chat_sessions,
            delete_chat_session,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
