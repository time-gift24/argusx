import { invoke as originalInvoke } from "@tauri-apps/api/core";

// Use real Tauri backend
const invoke = originalInvoke;

// ============================================================================
// Error Types
// ============================================================================

export interface ApiError {
  code: string;
  message: string;
}

// ============================================================================
// Types
// ============================================================================

export type TargetLevel = "step" | "sop";

export type ChecklistStatus = "active" | "inactive" | "draft";

export type SourceType = "ai" | "manual";

export type ExecStatus = "pending" | "success" | "apierror" | "parsefailed";

// ============================================================================
// Checklist Types
// ============================================================================

export interface ChecklistItem {
  id: number;
  name: string;
  prompt: string;
  target_level: TargetLevel;
  result_schema: Record<string, unknown> | null;
  version: number;
  status: ChecklistStatus;
  created_at: string;
  updated_at: string;
  created_by: number | null;
  updated_by: number | null;
  deleted_at: string | null;
}

export interface CreateChecklistItemInput {
  name: string;
  prompt: string;
  target_level: TargetLevel;
  result_schema?: Record<string, unknown>;
  version?: number;
  status: ChecklistStatus;
  created_by?: number;
}

export interface UpdateChecklistItemInput {
  id: number;
  name?: string;
  prompt?: string;
  target_level?: TargetLevel;
  result_schema?: Record<string, unknown>;
  version?: number;
  status?: ChecklistStatus;
  updated_by?: number;
}

export interface ChecklistFilter {
  status?: ChecklistStatus;
  target_level?: TargetLevel;
}

// ============================================================================
// GoldenSet Types
// ============================================================================

export interface GoldenSetItem {
  golden_set_id: number;
  checklist_item_id: number;
  sort_order: number;
  created_at: string;
}

export interface BindGoldenSetItemInput {
  golden_set_id: number;
  checklist_item_id: number;
  sort_order: number;
}

// ============================================================================
// CheckResult Types
// ============================================================================

export interface CheckResult {
  id: number;
  context_type: string;
  context_key: string;
  check_item_id: number;
  context_id?: number;
  source_type: SourceType;
  operator_id: string | null;
  result: Record<string, unknown> | null;
  is_pass: boolean;
  created_at: number;
}

export interface UpsertCheckResultInput {
  id?: number;
  context_type: string;
  context_key: string;
  check_item_id: number | null;
  source_type: SourceType;
  operator_id?: string;
  result?: Record<string, unknown>;
  is_pass?: boolean;
}

export interface CheckResultFilter {
  context_type?: string;
  context_key?: string;
  check_item_id?: number;
}

// ============================================================================
// AiExecutionLog Types
// ============================================================================

export interface AiExecutionLog {
  id: number;
  check_result_id: number | null;
  context_type: string;
  context_key: string;
  context_id?: number;
  check_item_id: number;
  model_provider: string | null;
  model_version: string;
  temperature: number | null;
  prompt_snapshot: string | null;
  raw_output: string | null;
  input_tokens: number;
  output_tokens: number;
  exec_status: ExecStatus;
  error_message: string | null;
  latency_ms: number;
  created_at: number;
}

export interface AppendAiExecutionLogInput {
  check_result_id?: number;
  context_type: string;
  context_key: string;
  check_item_id: number;
  model_provider?: string;
  model_version: string;
  temperature?: number;
  prompt_snapshot?: string;
  raw_output?: string;
  input_tokens?: number;
  output_tokens?: number;
  exec_status: ExecStatus;
  error_message?: string;
  latency_ms?: number;
}

export interface AiExecutionLogFilter {
  context_type?: string;
  context_key?: string;
  check_item_id?: number;
}

// ============================================================================
// API Functions
// ============================================================================

// Checklist API
export async function createChecklistItem(
  input: CreateChecklistItemInput
): Promise<ChecklistItem> {
  return invoke<ChecklistItem>("create_checklist_item", { input });
}

export async function updateChecklistItem(
  input: UpdateChecklistItemInput
): Promise<ChecklistItem> {
  return invoke<ChecklistItem>("update_checklist_item", { input });
}

export async function listChecklistItems(
  filter: ChecklistFilter = {}
): Promise<ChecklistItem[]> {
  return invoke<ChecklistItem[]>("list_checklist_items", { filter });
}

export async function getChecklistItem(id: number): Promise<ChecklistItem> {
  return invoke<ChecklistItem>("get_checklist_item", { id });
}

export async function deleteChecklistItem(id: number): Promise<void> {
  return invoke<void>("delete_checklist_item", { id });
}

// GoldenSet API
export async function bindGoldenSetItem(
  input: BindGoldenSetItemInput
): Promise<GoldenSetItem> {
  return invoke<GoldenSetItem>("bind_golden_set_item", { input });
}

export async function listGoldenSetItems(
  goldenSetId: number
): Promise<GoldenSetItem[]> {
  return invoke<GoldenSetItem[]>("list_golden_set_items", {
    goldenSetId,
  });
}

export async function unbindGoldenSetItem(
  goldenSetId: number,
  checklistItemId: number
): Promise<void> {
  return invoke<void>("unbind_golden_set_item", {
    input: {
      golden_set_id: goldenSetId,
      checklist_item_id: checklistItemId,
    },
  });
}

// CheckResult API
export async function upsertOrAppendCheckResult(
  input: UpsertCheckResultInput
): Promise<CheckResult> {
  return invoke<CheckResult>("upsert_or_append_check_result", { input });
}

export async function upsertCheckResult(
  input: UpsertCheckResultInput
): Promise<CheckResult> {
  return upsertOrAppendCheckResult(input);
}

export async function listCheckResults(
  filter: CheckResultFilter = {}
): Promise<CheckResult[]> {
  return invoke<CheckResult[]>("list_check_results", { filter });
}

// AiExecutionLog API
export async function appendAiExecutionLog(
  input: AppendAiExecutionLogInput
): Promise<AiExecutionLog> {
  return invoke<AiExecutionLog>("append_ai_execution_log", { input });
}

export async function listAiExecutionLogs(
  filter: AiExecutionLogFilter = {}
): Promise<AiExecutionLog[]> {
  return invoke<AiExecutionLog[]>("list_ai_execution_logs", { filter });
}

// ============== SOP Types ==============

export type SopStatus = "active" | "inactive" | "draft";

export interface Sop {
  id: number;
  sop_id: string;
  name: string;
  ticket_id: string | null;
  version: number;
  detect: Record<string, unknown> | null;
  handle: Record<string, unknown> | null;
  verification: Record<string, unknown> | null;
  rollback: Record<string, unknown> | null;
  status: SopStatus;
  created_at: string;
  updated_at: string;
}

export interface CreateSopInput {
  sop_id: string;
  name: string;
  ticket_id?: string;
  version?: number;
  detect?: Record<string, unknown>;
  handle?: Record<string, unknown>;
  verification?: Record<string, unknown>;
  rollback?: Record<string, unknown>;
  status: SopStatus;
}

export interface UpdateSopInput {
  id: number;
  sop_id?: string;
  name?: string;
  ticket_id?: string;
  version?: number;
  detect?: Record<string, unknown>;
  handle?: Record<string, unknown>;
  verification?: Record<string, unknown>;
  rollback?: Record<string, unknown>;
  status?: SopStatus;
}

export interface SopFilter {
  status?: SopStatus;
  ticket_id?: string;
}

export interface SopStep {
  id: number;
  sop_id: string;
  name: string;
  version: number;
  operation: Record<string, unknown> | null;
  verification: Record<string, unknown> | null;
  impact_analysis: Record<string, unknown> | null;
  rollback: Record<string, unknown> | null;
  created_at: string;
  updated_at: string;
}

export interface CreateSopStepInput {
  sop_id: string;
  name: string;
  version?: number;
  operation?: Record<string, unknown>;
  verification?: Record<string, unknown>;
  impact_analysis?: Record<string, unknown>;
  rollback?: Record<string, unknown>;
}

export interface UpdateSopStepInput {
  id: number;
  name?: string;
  version?: number;
  operation?: Record<string, unknown>;
  verification?: Record<string, unknown>;
  impact_analysis?: Record<string, unknown>;
  rollback?: Record<string, unknown>;
}

export interface SopStepFilter {
  sop_id?: string;
}

export interface SopAggregate {
  sop: Sop;
  detect_steps: SopStep[];
  handle_steps: SopStep[];
  verification_steps: SopStep[];
  rollback_steps: SopStep[];
}

// ============== SOP API Functions ==============

export async function createSop(input: CreateSopInput): Promise<Sop> {
  return invoke("create_sop", { input });
}

export async function updateSop(input: UpdateSopInput): Promise<Sop> {
  return invoke("update_sop", { input });
}

export async function listSops(filter: SopFilter = {}): Promise<Sop[]> {
  return invoke("list_sops", { filter });
}

export async function getSopAggregate(sop_id: string): Promise<SopAggregate> {
  return invoke("get_sop", { sop_id });
}

export async function getSop(id: number | string): Promise<Sop> {
  const sopId = typeof id === "number" ? `SOP-${id}` : id;
  const aggregate = await getSopAggregate(sopId);
  return aggregate.sop;
}

export async function deleteSop(id: number): Promise<void> {
  return invoke("delete_sop", { id });
}

export async function createSopStep(input: CreateSopStepInput): Promise<SopStep> {
  return invoke("create_sop_step", { input });
}

export async function updateSopStep(input: UpdateSopStepInput): Promise<SopStep> {
  return invoke("update_sop_step", { input });
}

export async function listSopSteps(filter: SopStepFilter = {}): Promise<SopStep[]> {
  return invoke("list_sop_steps", { filter });
}

export async function getSopStep(id: number): Promise<SopStep> {
  return invoke("get_sop_step", { id });
}

export async function deleteSopStep(id: number): Promise<void> {
  return invoke("delete_sop_step", { id });
}
