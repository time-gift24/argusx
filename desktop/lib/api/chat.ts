import { invoke } from "@tauri-apps/api/core";

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
    return await invoke("get_chat_messages", { sessionId });
  } catch (error) {
    throw new Error(`Failed to get chat messages: ${error}`);
  }
}
