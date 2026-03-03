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

export interface ChatTurnSummary {
  id: string;
  session_id: string;
  status: "running" | "done" | "failed" | "cancelled";
  final_message?: string;
  created_at: number;
  updated_at: number;
}

export interface GetChatMessagesOptions {
  range?: "window_24h" | "all";
  cursor?: number;
  limit?: number;
}

export interface UpdateChatSessionPayload {
  title: string;
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

export type ProviderId = "bigmodel" | "openai" | "anthropic";

export interface HeaderPair {
  key: string;
  value: string;
}

export interface ProviderRuntimeConfig {
  apiKey: string;
  baseUrl: string;
  models: string[];
  headers: HeaderPair[];
}

export interface ProviderConfigs {
  bigmodel: ProviderRuntimeConfig;
  openai: ProviderRuntimeConfig;
  anthropic: ProviderRuntimeConfig;
}

export interface LlmRuntimeConfig {
  defaultProvider?: ProviderId;
  providers: ProviderConfigs;
}

export interface AvailableModel {
  provider: ProviderId;
  model: string;
}

export interface StartAgentTurnPayload {
  sessionId: string;
  input: string;
  provider: ProviderId;
  model: string;
  attachments?: unknown[];
}

export interface StartAgentTurnResponse {
  turnId: string;
}

export interface RestoreTurnCheckpointPayload {
  sessionId: string;
  turnId: string;
}

export interface RestoreTurnCheckpointResponse {
  restoredTurnId: string;
  removedTurnIds: string[];
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

export async function updateChatSession(
  id: string,
  payload: UpdateChatSessionPayload
): Promise<ChatSession> {
  try {
    return await invoke("update_chat_session", { payload: { id, ...payload } });
  } catch (error) {
    throw new Error(`Failed to update chat session: ${error}`);
  }
}

export async function getChatMessages(
  sessionId: string,
  options?: GetChatMessagesOptions
): Promise<ChatMessage[]> {
  try {
    return await invoke("get_chat_messages", {
      sessionId,
      range: options?.range,
      cursor: options?.cursor,
      limit: options?.limit,
    });
  } catch (error) {
    throw new Error(`Failed to get chat messages: ${error}`);
  }
}

export async function getChatTurnSummaries(
  sessionId: string
): Promise<ChatTurnSummary[]> {
  try {
    return await invoke("get_chat_turn_summaries", { sessionId });
  } catch (error) {
    throw new Error(`Failed to get chat turn summaries: ${error}`);
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

export async function getLlmRuntimeConfig(): Promise<LlmRuntimeConfig> {
  try {
    return await invoke("get_llm_runtime_config");
  } catch (error) {
    throw new Error(`Failed to get llm runtime config: ${error}`);
  }
}

export async function setLlmRuntimeConfig(
  payload: LlmRuntimeConfig
): Promise<LlmRuntimeConfig> {
  try {
    return await invoke("set_llm_runtime_config", { payload });
  } catch (error) {
    throw new Error(`Failed to set llm runtime config: ${error}`);
  }
}

export async function clearLlmRuntimeConfig(): Promise<LlmRuntimeConfig> {
  try {
    return await invoke("clear_llm_runtime_config");
  } catch (error) {
    throw new Error(`Failed to clear llm runtime config: ${error}`);
  }
}

export async function listAvailableModels(): Promise<AvailableModel[]> {
  try {
    return await invoke("list_available_models");
  } catch (error) {
    throw new Error(`Failed to list available models: ${error}`);
  }
}

export async function cancelAgentTurn(turnId: string): Promise<void> {
  try {
    await invoke("cancel_agent_turn", { payload: { turnId } });
  } catch (error) {
    throw new Error(`Failed to cancel agent turn: ${error}`);
  }
}

export async function restoreTurnCheckpoint(
  payload: RestoreTurnCheckpointPayload
): Promise<RestoreTurnCheckpointResponse> {
  try {
    return await invoke("restore_turn_checkpoint", { payload });
  } catch (error) {
    throw new Error(`Failed to restore checkpoint: ${error}`);
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
