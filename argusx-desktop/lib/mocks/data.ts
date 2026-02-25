import type {
  ChecklistItem,
  GoldenSetItem,
  CheckResult,
  AiExecutionLog,
} from "@/lib/api/prompt-lab";

export const mockChecklistItems: ChecklistItem[] = [
  {
    id: 1,
    name: "Check JSON syntax",
    prompt: "Validate that the output is valid JSON",
    target_level: "step",
    result_schema: { type: "boolean" },
    version: 1,
    status: "active",
    created_at: "2024-01-01T00:00:00Z",
    updated_at: "2024-01-01T00:00:00Z",
    created_by: null,
    updated_by: null,
    deleted_at: null,
  },
  {
    id: 2,
    name: "Check response length",
    prompt: "Ensure response is between 10-500 characters",
    target_level: "step",
    result_schema: { type: "number", min: 10, max: 500 },
    version: 1,
    status: "active",
    created_at: "2024-01-02T00:00:00Z",
    updated_at: "2024-01-02T00:00:00Z",
    created_by: null,
    updated_by: null,
    deleted_at: null,
  },
];

export const mockGoldenSetItems: GoldenSetItem[] = [
  { golden_set_id: 1, checklist_item_id: 1, sort_order: 1, created_at: "2024-01-01T00:00:00Z" },
  { golden_set_id: 1, checklist_item_id: 2, sort_order: 2, created_at: "2024-01-01T00:00:00Z" },
];

export const mockCheckResults: CheckResult[] = [
  {
    id: 1,
    context_type: "prompt",
    context_key: "sop:SOP-1",
    check_item_id: 1,
    source_type: "ai",
    operator_id: "user123",
    result: { valid: true },
    is_pass: true,
    created_at: 1704067200000,
  },
];

export const mockAiExecutionLogs: AiExecutionLog[] = [
  {
    id: 1,
    check_result_id: 1,
    context_type: "prompt",
    context_key: "sop:SOP-1",
    check_item_id: 1,
    model_provider: "openai",
    model_version: "gpt-4",
    temperature: 0.7,
    prompt_snapshot: "Validate this JSON: {\"key\": \"value\"}",
    raw_output: '{"valid": true}',
    input_tokens: 50,
    output_tokens: 10,
    exec_status: "success",
    error_message: null,
    latency_ms: 1500,
    created_at: 1704067200000,
  },
];
