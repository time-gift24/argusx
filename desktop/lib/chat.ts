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

export interface DesktopTurnEvent {
  turnId: string;
  type: string;
  data: Record<string, unknown>;
}

export async function startTurn(
  input: StartTurnInput
): Promise<StartTurnResult> {
  return invoke<StartTurnResult>("start_turn", { input });
}

export async function cancelTurn(turnId: string): Promise<void> {
  await invoke("cancel_turn", { turnId });
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
    startTurn,
    subscribe,
  };
}
