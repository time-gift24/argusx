// Type definitions for the application

// ============================================
// AI Types (replacing Vercel AI SDK types)
// ============================================

export type MessageRole = "user" | "assistant" | "system" | "tool";

export interface UIMessage {
  role: MessageRole;
  content: string;
  id: string;
}

export type ToolState =
  | "approval-requested"
  | "approval-responded"
  | "input-available"
  | "input-streaming"
  | "output-available"
  | "output-denied"
  | "output-error";

export interface ToolUIPart {
  type: "tool";
  toolName: string;
  toolCallId: string;
  state: ToolState;
  input?: Record<string, unknown>;
  output?: unknown;
  errorText?: string;
}

export interface DynamicToolUIPart {
  type: "dynamic-tool";
  toolName: string;
  toolCallId: string;
  state: ToolState;
  input?: Record<string, unknown>;
  output?: unknown;
  errorText?: string;
}

export interface FileUIPart {
  type: "file";
  mimeType?: string;
  url?: string;
  filename?: string;
  mediaType?: string;
  id?: string;
}

export interface SourceDocumentUIPart {
  type: "source" | "source-document";
  sourceType: "url" | "document";
  url?: string;
  title?: string;
  mediaType?: string;
  filename?: string;
}

export type ChatStatus = "submitted" | "streaming" | "ready" | "error";

export interface LanguageModelUsage {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  inputTokens?: number;
  outputTokens?: number;
  reasoningTokens?: number;
  cachedInputTokens?: number;
}

export interface Experimental_GeneratedImage {
  url?: string;
  alt?: string;
  pending: boolean;
  base64?: string;
  mediaType?: string;
  uint8Array?: Uint8Array;
}

export interface TranscriptionSegment {
  start: number;
  end: number;
  text: string;
  startSecond: number;
  endSecond: number;
}

export interface Experimental_TranscriptionResult {
  transcript: string;
  language?: string;
  segments: TranscriptionSegment[];
}

export interface Experimental_SpeechResult {
  text: string;
  isLoading: boolean;
  audio?: string;
  base64?: string;
  mediaType?: string;
}

// Speech result audio data as base64 string with mediaType
export type SpeechResultAudio = {
  base64: string;
  mediaType: string;
};

export interface Tool {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
  inputSchema?: Record<string, unknown>;
  jsonSchema?: Record<string, unknown>;
}

// ============================================
// Chat Types
// ============================================

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: Date;
}

export interface ChatSession {
  id: string;
  title: string;
  messages: ChatMessage[];
  createdAt: Date;
  updatedAt: Date;
}

export interface User {
  id: string;
  name: string;
  email: string;
  avatar?: string;
}
