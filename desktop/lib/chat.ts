import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type TurnTargetKind = "agent" | "workflow";

export interface StartTurnInput {
  prompt: string;
  targetKind: TurnTargetKind;
  targetId: string;
}

export interface StartTurnResult {
  turnId: string;
}

export type HydratedToolCallStatus =
  | "running"
  | "success"
  | "failed"
  | "timed_out"
  | "denied"
  | "cancelled";

export interface HydratedToolCall {
  callId: string;
  name: string;
  argumentsJson: string;
  outputSummary: string | null;
  errorSummary: string | null;
  status: HydratedToolCallStatus;
}

export type HydratedTurnStatus = "completed" | "cancelled" | "failed";

export interface HydratedChatTurn {
  turnId: string;
  prompt: string;
  assistantText: string;
  reasoningText: string;
  status: HydratedTurnStatus;
  error: string | null;
  latestPlan: Record<string, unknown> | null;
  toolCalls: HydratedToolCall[];
}

export interface DesktopTurnEvent {
  turnId: string;
  type: string;
  data: Record<string, unknown>;
}

export type PermissionDecision = "allow" | "deny";

export interface ToolCallPermissionRequestedEventData {
  requestId: string;
  toolCallId: string;
}

export interface ToolCallPermissionResolvedEventData {
  requestId: string;
  decision: PermissionDecision;
}

export async function startTurn(
  input: StartTurnInput
): Promise<StartTurnResult> {
  return invoke<StartTurnResult>("start_turn", { input });
}

export async function loadActiveChatThread(): Promise<HydratedChatTurn[]> {
  return invoke<HydratedChatTurn[]>("load_active_chat_thread");
}

export async function cancelTurn(turnId: string): Promise<void> {
  await invoke("cancel_turn", { turnId });
}

export async function resolveTurnPermission(
  turnId: string,
  requestId: string,
  decision: PermissionDecision
): Promise<void> {
  await invoke("resolve_turn_permission", {
    decision,
    requestId,
    turnId,
  });
}

export async function subscribe(
  callback: (event: DesktopTurnEvent) => void
): Promise<UnlistenFn> {
  return listen<DesktopTurnEvent>("turn-event", (event) => {
    callback(event.payload);
  });
}

export function useTurn() {
  return {
    cancelTurn,
    loadActiveChatThread,
    resolveTurnPermission,
    startTurn,
    subscribe,
  };
}
