import {
  mockChecklistItems,
  mockGoldenSetItems,
  mockCheckResults,
  mockAiExecutionLogs,
} from "./data";
import type {
  ChecklistItem,
  GoldenSetItem,
  CheckResult,
  AiExecutionLog,
  ChecklistFilter,
  CheckResultFilter,
  AiExecutionLogFilter,
  CreateChecklistItemInput,
  UpdateChecklistItemInput,
  BindGoldenSetItemInput,
  UpsertCheckResultInput,
  AppendAiExecutionLogInput,
} from "@/lib/api/prompt-lab";

// In-memory data store for mock
let checklistItems: ChecklistItem[] = [...mockChecklistItems];
let goldenSetItems: GoldenSetItem[] = [...mockGoldenSetItems];
let checkResults: CheckResult[] = [...mockCheckResults];
let aiExecutionLogs: AiExecutionLog[] = [...mockAiExecutionLogs];
let nextChecklistId = 3;

// Mock invoke function that simulates Tauri IPC calls
export async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  switch (cmd) {
    // Checklist mocks
    case "create_checklist_item": {
      const input = args?.input as CreateChecklistItemInput;
      const item: ChecklistItem = {
        id: nextChecklistId++,
        name: input.name,
        prompt: input.prompt,
        target_level: input.target_level,
        result_schema: input.result_schema ?? null,
        version: input.version ?? 1,
        status: input.status,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        created_by: input.created_by ?? null,
        updated_by: null,
        deleted_at: null,
      };
      checklistItems.push(item);
      return item as T;
    }

    case "update_checklist_item": {
      const input = args?.input as UpdateChecklistItemInput;
      const index = checklistItems.findIndex((i) => i.id === input.id);
      if (index === -1) throw new Error("Not found");
      checklistItems[index] = {
        ...checklistItems[index],
        ...input,
        updated_at: new Date().toISOString(),
      };
      return checklistItems[index] as T;
    }

    case "delete_checklist_item": {
      const id = args?.id as number;
      checklistItems = checklistItems.filter((i) => i.id !== id);
      return undefined as T;
    }

    case "list_checklist_items": {
      const filter = args?.filter as ChecklistFilter | undefined;
      if (filter?.status) {
        return checklistItems.filter((i) => i.status === filter.status) as T;
      }
      return checklistItems as T;
    }

    // GoldenSet mocks
    case "bind_golden_set_item": {
      const input = args?.input as BindGoldenSetItemInput;
      const item: GoldenSetItem = {
        golden_set_id: input.golden_set_id,
        checklist_item_id: input.checklist_item_id,
        sort_order: input.sort_order,
        created_at: new Date().toISOString(),
      };
      goldenSetItems.push(item);
      return item as T;
    }

    case "unbind_golden_set_item": {
      const input = args?.input as { golden_set_id: number; checklist_item_id: number };
      goldenSetItems = goldenSetItems.filter(
        (i) => !(i.golden_set_id === input.golden_set_id && i.checklist_item_id === input.checklist_item_id)
      );
      return undefined as T;
    }

    case "list_golden_set_items": {
      const golden_set_id = args?.golden_set_id as number;
      return goldenSetItems.filter((i) => i.golden_set_id === golden_set_id) as T;
    }

    // CheckResult mocks
    case "upsert_or_append_check_result":
    case "upsert_check_result": {
      const input = args?.input as UpsertCheckResultInput;
      const isPass =
        input.source_type === "manual"
          ? (input.is_pass ?? true)
          : (input.is_pass ?? false);

      if (input.source_type === "manual" && input.check_item_id != null) {
        const existingIndex = checkResults.findIndex(
          (r) =>
            r.source_type === "manual" &&
            r.context_type === input.context_type &&
            r.context_key === input.context_key &&
            r.check_item_id === input.check_item_id
        );
        if (existingIndex >= 0) {
          checkResults[existingIndex] = {
            ...checkResults[existingIndex],
            operator_id: input.operator_id ?? null,
            result: input.result ?? null,
            is_pass: isPass,
            created_at: Date.now(),
          };
          return checkResults[existingIndex] as T;
        }
      }

      const item: CheckResult = {
        id: input.id ?? checkResults.length + 1,
        context_type: input.context_type,
        context_key: input.context_key,
        check_item_id: input.check_item_id ?? -1,
        source_type: input.source_type,
        operator_id: input.operator_id ?? null,
        result: input.result ?? null,
        is_pass: isPass,
        created_at: Date.now(),
      };
      checkResults.push(item);
      return item as T;
    }

    case "list_check_results": {
      const filter = args?.filter as CheckResultFilter | undefined;
      let results = checkResults;
      if (filter?.context_type) {
        results = results.filter((r) => r.context_type === filter.context_type);
      }
      if (filter?.context_key) {
        results = results.filter((r) => r.context_key === filter.context_key);
      }
      if (filter?.check_item_id) {
        results = results.filter((r) => r.check_item_id === filter.check_item_id);
      }
      return results as T;
    }

    // AiExecutionLog mocks
    case "append_ai_execution_log": {
      const input = args?.input as AppendAiExecutionLogInput;
      const item: AiExecutionLog = {
        id: aiExecutionLogs.length + 1,
        check_result_id: input.check_result_id ?? null,
        context_type: input.context_type,
        context_key: input.context_key,
        check_item_id: input.check_item_id,
        model_provider: input.model_provider ?? null,
        model_version: input.model_version,
        temperature: input.temperature ?? null,
        prompt_snapshot: input.prompt_snapshot ?? null,
        raw_output: input.raw_output ?? null,
        input_tokens: input.input_tokens ?? 0,
        output_tokens: input.output_tokens ?? 0,
        exec_status: input.exec_status,
        error_message: input.error_message ?? null,
        latency_ms: input.latency_ms ?? 0,
        created_at: Date.now(),
      };
      aiExecutionLogs.push(item);
      return item as T;
    }

    case "list_ai_execution_logs": {
      const filter = args?.filter as AiExecutionLogFilter | undefined;
      let logs = aiExecutionLogs;
      if (filter?.context_type) {
        logs = logs.filter((l) => l.context_type === filter.context_type);
      }
      if (filter?.context_key) {
        logs = logs.filter((l) => l.context_key === filter.context_key);
      }
      if (filter?.check_item_id) {
        logs = logs.filter((l) => l.check_item_id === filter.check_item_id);
      }
      return logs as T;
    }

    default:
      throw new Error(`Unknown mock command: ${cmd}`);
  }
}

export function setupPromptLabMocks() {
  // This function is kept for compatibility
  // The actual mocking is done through mockInvoke function
  console.log("PromptLab mocks initialized");
}

export function clearMocks() {
  checklistItems = [...mockChecklistItems];
  goldenSetItems = [...mockGoldenSetItems];
  checkResults = [...mockCheckResults];
  aiExecutionLogs = [...mockAiExecutionLogs];
}
