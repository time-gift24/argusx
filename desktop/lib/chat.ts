import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type TurnTargetKind = "agent" | "workflow";

export type StartConversationInput = {
  prompt: string;
  targetId: string;
  targetKind: TurnTargetKind;
};

export type ContinueConversationInput = {
  conversationId: string;
  prompt: string;
};

export type CancelConversationInput = {
  conversationId: string;
};

export type ThreadStatus = "idle" | "running" | "restartable";

export type CreateConversationThreadInput = {
  title: string;
  targetId: string;
  targetKind: TurnTargetKind;
};

export type SwitchConversationThreadInput = {
  conversationId: string;
};

export type RestartConversationInput = {
  conversationId: string;
};

export type ConversationTurnStarted = {
  conversationId: string;
  turnId: string;
};

export type ConversationThreadSummary = {
  conversationId: string;
  title: string;
  targetId: string;
  targetKind: TurnTargetKind;
  updatedAtMs: number;
  status: ThreadStatus;
  isActive: boolean;
};

export type DesktopTurnEvent = {
  conversationId: string;
  turnId: string;
  type: string;
  data: Record<string, unknown>;
};

export const CHAT_AGENTS = [
  {
    description: "Review a change set with an engineering lens",
    id: "reviewer",
    label: "Code Reviewer",
  },
  {
    description: "Break ambiguous work into concrete steps",
    id: "planner",
    label: "Planner",
  },
] as const;

export const CHAT_WORKFLOWS = [
  {
    description: "Draft a design-oriented implementation brief",
    id: "design",
    label: "Write Design",
  },
  {
    description: "Prepare the task for a focused implementation pass",
    id: "execute",
    label: "Execute Plan",
  },
] as const;

export async function startConversation(
  input: StartConversationInput
): Promise<ConversationTurnStarted> {
  return invoke("start_conversation", { input });
}

export async function continueConversation(
  input: ContinueConversationInput
): Promise<ConversationTurnStarted> {
  return invoke("continue_conversation", { input });
}

export async function cancelConversation(
  input: CancelConversationInput
): Promise<void> {
  await invoke("cancel_conversation", { input });
}

export async function createConversationThread(
  input: CreateConversationThreadInput
): Promise<ConversationThreadSummary> {
  return invoke("create_conversation_thread", { input });
}

export async function listConversationThreads(): Promise<
  ConversationThreadSummary[]
> {
  return invoke("list_conversation_threads");
}

export async function switchConversationThread(
  input: SwitchConversationThreadInput
): Promise<ConversationThreadSummary> {
  return invoke("switch_conversation_thread", { input });
}

export async function restartConversation(
  input: RestartConversationInput
): Promise<ConversationTurnStarted> {
  return invoke("restart_conversation", { input });
}

export function subscribeToTurnEvents(
  onEvent: (event: DesktopTurnEvent) => void
): Promise<() => void> {
  return listen<DesktopTurnEvent>("turn-event", (event) => {
    onEvent(event.payload);
  });
}
