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
  context_id: number;
  check_item_id: number;
  source_type: SourceType;
  operator_id: string | null;
  result: Record<string, unknown> | null;
  is_pass: boolean;
  created_at: string;
}

export interface UpsertCheckResultInput {
  id?: number;
  context_type: string;
  context_id: number;
  check_item_id: number;
  source_type: SourceType;
  operator_id?: string;
  result?: Record<string, unknown>;
  is_pass: boolean;
}

export interface CheckResultFilter {
  context_type?: string;
  context_id?: number;
  check_item_id?: number;
}

// ============================================================================
// AiExecutionLog Types
// ============================================================================

export interface AiExecutionLog {
  id: number;
  check_result_id: number | null;
  context_type: string;
  context_id: number;
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
  created_at: string;
}

export interface AppendAiExecutionLogInput {
  check_result_id?: number;
  context_type: string;
  context_id: number;
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
  context_id?: number;
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
    golden_set_id: goldenSetId,
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
export async function upsertCheckResult(
  input: UpsertCheckResultInput
): Promise<CheckResult> {
  return invoke<CheckResult>("upsert_check_result", { input });
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
