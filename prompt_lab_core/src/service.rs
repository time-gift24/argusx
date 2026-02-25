use std::sync::Arc;
use std::{collections::HashMap, fmt::Display};

use serde_json::Value;

use crate::domain::*;
use crate::error::{PromptLabError, Result};
use crate::repository::PromptLabRepository;

#[derive(Clone)]
pub struct ChecklistService {
    repo: Arc<PromptLabRepository>,
}

impl ChecklistService {
    pub fn new(repo: Arc<PromptLabRepository>) -> Self {
        Self { repo }
    }

    pub async fn create(&self, input: CreateChecklistItemInput) -> Result<ChecklistItem> {
        validate_non_empty("name", &input.name)?;
        validate_non_empty("prompt", &input.prompt)?;
        validate_result_schema(input.result_schema.as_ref())?;
        self.repo.create_checklist_item(input).await
    }

    pub async fn update(&self, input: UpdateChecklistItemInput) -> Result<ChecklistItem> {
        if let Some(name) = input.name.as_ref() {
            validate_non_empty("name", name)?;
        }
        if let Some(prompt) = input.prompt.as_ref() {
            validate_non_empty("prompt", prompt)?;
        }
        validate_result_schema(input.result_schema.as_ref())?;
        self.repo.update_checklist_item(input).await
    }

    pub async fn list(&self, filter: ChecklistFilter) -> Result<Vec<ChecklistItem>> {
        self.repo.list_checklist_items(filter).await
    }

    pub async fn get(&self, id: i64) -> Result<ChecklistItem> {
        self.repo.get_checklist_item(id).await
    }

    pub async fn soft_delete(&self, id: i64) -> Result<()> {
        self.repo.soft_delete_checklist_item(id).await
    }
}

#[derive(Clone)]
pub struct GoldenSetService {
    repo: Arc<PromptLabRepository>,
}

impl GoldenSetService {
    pub fn new(repo: Arc<PromptLabRepository>) -> Self {
        Self { repo }
    }

    pub async fn bind(&self, input: BindGoldenSetItemInput) -> Result<GoldenSetItem> {
        self.repo.bind_golden_set_item(input).await
    }

    pub async fn list(&self, golden_set_id: i64) -> Result<Vec<GoldenSetItem>> {
        self.repo.list_golden_set_items(golden_set_id).await
    }

    pub async fn unbind(&self, golden_set_id: i64, checklist_item_id: i64) -> Result<()> {
        self.repo
            .unbind_golden_set_item(golden_set_id, checklist_item_id)
            .await
    }
}

#[derive(Clone)]
pub struct CheckResultService {
    repo: Arc<PromptLabRepository>,
}

impl CheckResultService {
    pub fn new(repo: Arc<PromptLabRepository>) -> Self {
        Self { repo }
    }

    pub async fn upsert(&self, input: UpsertCheckResultInput) -> Result<CheckResult> {
        self.upsert_or_append(input).await
    }

    pub async fn upsert_or_append(&self, input: UpsertCheckResultInput) -> Result<CheckResult> {
        validate_non_empty("context_type", &input.context_type)?;
        validate_non_empty("context_key", &input.context_key)?;
        self.repo.upsert_or_append_check_result(input).await
    }

    pub async fn list(&self, filter: CheckResultFilter) -> Result<Vec<CheckResult>> {
        self.repo.list_check_results(filter).await
    }
}

#[derive(Clone)]
pub struct AiLogService {
    repo: Arc<PromptLabRepository>,
}

impl AiLogService {
    pub fn new(repo: Arc<PromptLabRepository>) -> Self {
        Self { repo }
    }

    pub async fn append(&self, input: AppendAiExecutionLogInput) -> Result<AiExecutionLog> {
        validate_non_empty("context_type", &input.context_type)?;
        validate_non_empty("context_key", &input.context_key)?;
        validate_non_empty("model_version", &input.model_version)?;
        self.repo.append_ai_execution_log(input).await
    }

    pub async fn list(&self, filter: AiExecutionLogFilter) -> Result<Vec<AiExecutionLog>> {
        self.repo.list_ai_execution_logs(filter).await
    }
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(PromptLabError::InvalidInput(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn validate_result_schema(schema: Option<&Value>) -> Result<()> {
    if let Some(value) = schema {
        if !value.is_object() {
            return Err(PromptLabError::InvalidInput(
                "result_schema must be a JSON object".to_string(),
            ));
        }
    }
    Ok(())
}

#[derive(Clone)]
pub struct SopService {
    repo: Arc<PromptLabRepository>,
}

impl SopService {
    pub fn new(repo: Arc<PromptLabRepository>) -> Self {
        Self { repo }
    }

    pub async fn create_sop(&self, input: CreateSopInput) -> Result<Sop> {
        self.repo.create_sop(input).await
    }

    pub async fn update_sop(&self, input: UpdateSopInput) -> Result<Sop> {
        self.repo.update_sop(input).await
    }

    pub async fn list_sops(&self, filter: SopFilter) -> Result<Vec<Sop>> {
        self.repo.list_sops(filter).await
    }

    pub async fn get_sop(&self, id: i64) -> Result<Sop> {
        self.repo.get_sop(id).await
    }

    pub async fn get_sop_by_sop_id(&self, sop_id: &str) -> Result<Sop> {
        self.repo.get_sop_by_sop_id(sop_id).await
    }

    pub async fn get_sop_aggregate_by_sop_id(&self, sop_id: &str) -> Result<SopAggregate> {
        let sop = self.repo.get_sop_by_sop_id(sop_id).await?;
        let steps = self
            .repo
            .list_sop_steps(SopStepFilter {
                sop_id: Some(sop.sop_id.clone()),
            })
            .await?;
        let step_map: HashMap<i64, SopStep> = steps.into_iter().map(|step| (step.id, step)).collect();

        let detect_steps = collect_stage_steps(&sop.detect, &step_map, "detect")?;
        let handle_steps = collect_stage_steps(&sop.handle, &step_map, "handle")?;
        let verification_steps = collect_stage_steps(&sop.verification, &step_map, "verification")?;
        let rollback_steps = collect_stage_steps(&sop.rollback, &step_map, "rollback")?;

        Ok(SopAggregate {
            sop,
            detect_steps,
            handle_steps,
            verification_steps,
            rollback_steps,
        })
    }

    pub async fn delete_sop(&self, id: i64) -> Result<()> {
        self.repo.delete_sop(id).await
    }

    pub async fn create_sop_step(&self, input: CreateSopStepInput) -> Result<SopStep> {
        self.repo.create_sop_step(input).await
    }

    pub async fn update_sop_step(&self, input: UpdateSopStepInput) -> Result<SopStep> {
        self.repo.update_sop_step(input).await
    }

    pub async fn list_sop_steps(&self, filter: SopStepFilter) -> Result<Vec<SopStep>> {
        self.repo.list_sop_steps(filter).await
    }

    pub async fn get_sop_step(&self, id: i64) -> Result<SopStep> {
        self.repo.get_sop_step(id).await
    }

    pub async fn delete_sop_step(&self, id: i64) -> Result<()> {
        self.repo.delete_sop_step(id).await
    }
}

fn collect_stage_steps(
    snapshot: &Option<Value>,
    step_map: &HashMap<i64, SopStep>,
    stage: impl Display,
) -> Result<Vec<SopStep>> {
    let Some(snapshot) = snapshot else {
        return Ok(Vec::new());
    };
    let step_refs: Vec<SopStepRef> = serde_json::from_value(snapshot.clone())?;
    let mut ordered = Vec::with_capacity(step_refs.len());
    for step_ref in step_refs {
        let step = step_map
            .get(&step_ref.sop_step_id)
            .ok_or(PromptLabError::NotFound {
                entity: "sop_steps",
                id: step_ref.sop_step_id,
            })?
            .clone();
        ordered.push(step);
    }
    if ordered.is_empty() && !snapshot.is_null() {
        return Err(PromptLabError::InvalidInput(format!(
            "invalid {stage} snapshot references"
        )));
    }
    Ok(ordered)
}
