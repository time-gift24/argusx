import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface ChatSession {
  id: string;
  title: string;
  color: string;
  created_at: number;
  updated_at: number;
  status: "active" | "idle" | "archived";
}

export interface ChatMessage {
  id: string;
  session_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  created_at: number;
}

export type AgentStreamSource = "run" | "ui";

export interface AgentEventPayload {
  type: string;
  [key: string]: unknown;
}

export interface AgentStreamEnvelope {
  sessionId: string;
  turnId: string;
  source: AgentStreamSource;
  seq: number;
  ts: number;
  event: AgentEventPayload;
}

export interface StartAgentTurnPayload {
  sessionId: string;
  input: string;
  model?: string;
  attachments?: unknown[];
}

export interface StartAgentTurnResponse {
  turnId: string;
}

export async function createChatSession(title?: string): Promise<ChatSession> {
  try {
    return await invoke("create_chat_session", { title });
  } catch (error) {
    throw new Error(`Failed to create chat session: ${error}`);
  }
}

export async function listChatSessions(): Promise<ChatSession[]> {
  try {
    return await invoke("list_chat_sessions");
  } catch (error) {
    throw new Error(`Failed to list chat sessions: ${error}`);
  }
}

export async function deleteChatSession(id: string): Promise<void> {
  try {
    return await invoke("delete_chat_session", { id });
  } catch (error) {
    throw new Error(`Failed to delete chat session: ${error}`);
  }
}

export async function getChatMessages(sessionId: string): Promise<ChatMessage[]> {
  try {
    return await invoke("get_chat_messages", { session_id: sessionId });
  } catch (error) {
    throw new Error(`Failed to get chat messages: ${error}`);
  }
}

export async function startAgentTurn(
  payload: StartAgentTurnPayload
): Promise<StartAgentTurnResponse> {
  try {
    return await invoke("start_agent_turn", { payload });
  } catch (error) {
    throw new Error(`Failed to start agent turn: ${error}`);
  }
}

export async function cancelAgentTurn(turnId: string): Promise<void> {
  try {
    await invoke("cancel_agent_turn", { payload: { turnId } });
  } catch (error) {
    throw new Error(`Failed to cancel agent turn: ${error}`);
  }
}

export async function listenAgentStream(
  handler: (envelope: AgentStreamEnvelope) => void
): Promise<UnlistenFn> {
  // Return no-op if not running in Tauri (SSR or non-Tauri environment)
  if (!isTauri()) {
    return () => {};
  }
  return listen<AgentStreamEnvelope>("agent:stream", (event) => {
    handler(event.payload);
  });
}
