import type {
  AgentEventPayload,
  AgentStreamEnvelope,
  ChatMessage as BackendChatMessage,
  ChatSession as BackendChatSession,
  ChatTurnSummary as BackendChatTurnSummary,
  GetChatMessagesOptions,
} from "@/lib/api/chat";
import {
  createChatSession as createChatSessionApi,
  deleteChatSession as deleteChatSessionApi,
  getChatMessages,
  getChatTurnSummaries,
  listChatSessions,
  updateChatSession as updateChatSessionApi,
} from "@/lib/api/chat";
import { trimChatCacheToBudget } from "@/lib/stores/chat-cache-budget";
import type { ToolState } from "@/types";

import { create } from "zustand";

export type ChatStatus =
  | "wait-input"
  | "await-input"
  | "thinking"
  | "tool-call"
  | "outputing";

export interface ChatSession {
  id: string;
  title: string;
  color: string;
  status: ChatStatus;
  createdAt: number;
  updatedAt: number;
}

export interface ChatMessage {
  id: string;
  sessionId: string;
  role: "user" | "assistant" | "system";
  content: string;
  createdAt: number;
}

export interface ReasoningVM {
  isStreaming: boolean;
  isExpanded: boolean;
  preview: string;
  text: string;
  charCount: number;
  truncated: boolean;
  updatedAt: number;
  status: "idle" | "started" | "streaming" | "completed" | "error";
}

export interface ToolCallVM {
  callId: string;
  toolName: string;
  state: ToolState;
  input?: Record<string, unknown>;
  output?: unknown;
  errorText?: string;
  updatedAt: number;
}

export interface SubAgentToolVM {
  callId: string;
  toolName: string;
  status: string;
  updatedAt: number;
}

export interface SubAgentVM {
  threadId: string;
  agentName: string;
  status: string;
  tools: SubAgentToolVM[];
  errorText?: string;
  updatedAt: number;
}

export interface QueueItemVM {
  callId: string;
  toolName: string;
  status: "waiting" | "running" | "completed" | "failed";
  updatedAt: number;
}

export interface QueueVM {
  items: QueueItemVM[];
}

export interface TerminalVM {
  stdout: string;
  stderr: string;
  output: string;
  isStreaming: boolean;
  exitCode?: number;
  durationMs?: number;
  updatedAt: number;
}

export interface TaskVM {
  id: string;
  title: string;
  description?: string;
  status: "pending" | "completed";
}

export interface PlanVM {
  title: string;
  description?: string;
  tasks: TaskVM[];
  isStreaming: boolean;
}

export interface TodoQueueItemVM {
  id: string;
  title: string;
  description?: string;
  status: "pending" | "in_progress" | "blocked" | "completed" | "failed";
}

export interface TodoQueueVM {
  todos: TodoQueueItemVM[];
  updatedAt: number;
}

export type AgentTurnStatus =
  | "started"
  | "streaming"
  | "done"
  | "failed"
  | "cancelled";

export type TurnProcessSectionKey =
  | "reasoning"
  | "plan"
  | "tools"
  | "terminal";

export interface TurnUiState {
  processExpanded: boolean;
  sectionExpanded: Partial<Record<TurnProcessSectionKey, boolean>>;
  codeExpanded: Record<string, boolean>;
}

export interface AgentTurnVM {
  id: string;
  sessionId: string;
  requestMessageId?: string;
  createdAt: number;
  updatedAt: number;
  status: AgentTurnStatus;
  assistantText: string;
  reasoning: ReasoningVM;
  tools: ToolCallVM[];
  subAgents: SubAgentVM[];
  queue: QueueVM;
  terminal: TerminalVM;
  plan?: PlanVM;
  planSource?: "structured" | "reasoning-fallback";
  todoQueue?: TodoQueueVM;
  error?: string;
  lastSeq: number;
}

interface ChatState {
  sessions: ChatSession[];
  currentSessionId: string | null;
  messages: Record<string, ChatMessage[]>;
  turns: Record<string, AgentTurnVM[]>;
  turnUiState: Record<string, Record<string, TurnUiState>>;
  scrollToBottomSignal: Record<string, number>;
  cacheBytes: number;

  bootstrap: () => Promise<void>;
  createSession: () => Promise<string>;
  deleteSession: (id: string) => Promise<void>;
  loadSessionMessages: (
    sessionId: string,
    options?: GetChatMessagesOptions
  ) => Promise<void>;
  loadSessionTurns: (sessionId: string) => Promise<void>;
  loadFullSessionMessages: (sessionId: string, limit?: number) => Promise<void>;
  clearSessionCache: (sessionId: string) => void;
  updateSession: (
    id: string,
    updates: Partial<Pick<ChatSession, "title">>
  ) => Promise<void>;
  setCurrentSession: (id: string) => void;
  requestScrollToBottom: (sessionId: string) => void;
  addMessage: (
    sessionId: string,
    message: Omit<ChatMessage, "id" | "sessionId" | "createdAt">
  ) => string;
  updateSessionStatus: (id: string, status: ChatStatus) => void;
  ensureAgentTurn: (sessionId: string, turnId: string, requestMessageId?: string) => void;
  setReasoningExpanded: (sessionId: string, turnId: string, expanded: boolean) => void;
  setTurnProcessExpanded: (sessionId: string, turnId: string, expanded: boolean) => void;
  setTurnSectionExpanded: (
    sessionId: string,
    turnId: string,
    section: TurnProcessSectionKey,
    expanded: boolean
  ) => void;
  setTurnCodeExpanded: (
    sessionId: string,
    turnId: string,
    codeBlockId: string,
    expanded: boolean
  ) => void;
  restoreToCheckpoint: (
    sessionId: string,
    restoredTurnId: string,
    removedTurnIds: string[]
  ) => void;
  applyAgentStreamEnvelope: (envelope: AgentStreamEnvelope) => void;
}

const COLORS = ["blue", "emerald", "amber", "violet", "rose"];
const CACHE_BUDGET_BYTES = 64 * 1024 * 1024;
const DEFAULT_LOAD_LIMIT = 300;
const REASONING_CHAR_LIMIT = 24_000;
const COMPLETED_TASK_STATUSES = new Set(["completed", "done", "success", "succeeded", "finished"]);
const STRUCTURED_PLAN_EVENT_TYPES = new Set([
  "plan_started",
  "plan_updated",
  "plan_completed",
  "plan_delta",
  "plan_snapshot",
]);
const STRUCTURED_TASK_EVENT_TYPES = new Set([
  "task_started",
  "task_updated",
  "task_completed",
  "task_progress",
  "task_snapshot",
]);

const createEmptyTurn = (
  sessionId: string,
  turnId: string,
  requestMessageId?: string
): AgentTurnVM => {
  const now = Date.now();
  return {
    id: turnId,
    sessionId,
    requestMessageId,
    createdAt: now,
    updatedAt: now,
    status: "started",
    assistantText: "",
    reasoning: {
      isStreaming: false,
      isExpanded: false,
      preview: "",
      text: "",
      charCount: 0,
      truncated: false,
      updatedAt: now,
      status: "idle",
    },
    tools: [],
    subAgents: [],
    queue: { items: [] },
    terminal: {
      stdout: "",
      stderr: "",
      output: "",
      isStreaming: false,
      updatedAt: now,
    },
    lastSeq: 0,
  };
};

const createDefaultTurnUiState = (): TurnUiState => ({
  processExpanded: true,
  sectionExpanded: {},
  codeExpanded: {},
});

const toPreview = (text: string): string => {
  const compact = text.replace(/\s+/g, " ").trim();
  // Use code point count for consistent handling of emoji/CJK
  const codePoints = Array.from(compact);
  if (codePoints.length <= 180) {
    return compact;
  }
  return `${codePoints.slice(0, 180).join("")}...`;
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const normalizeTaskStatus = (status: unknown): TaskVM["status"] => {
  if (typeof status !== "string") {
    return "pending";
  }
  return COMPLETED_TASK_STATUSES.has(status.toLowerCase()) ? "completed" : "pending";
};

const normalizeTask = (raw: unknown, fallbackIndex: number): TaskVM | undefined => {
  if (typeof raw === "string") {
    const title = raw.trim();
    if (!title) {
      return undefined;
    }
    return {
      id: `task-${fallbackIndex + 1}`,
      title,
      status: "pending",
    };
  }

  if (!isRecord(raw)) {
    return undefined;
  }

  const titleCandidate = raw.title ?? raw.name ?? raw.label;
  const title =
    typeof titleCandidate === "string" ? titleCandidate.trim() : "";
  if (!title) {
    return undefined;
  }

  const idCandidate = raw.id ?? raw.task_id ?? raw.taskId;
  const id = typeof idCandidate === "string" && idCandidate.trim().length > 0
    ? idCandidate
    : `task-${fallbackIndex + 1}`;
  const description =
    typeof raw.description === "string" && raw.description.trim().length > 0
      ? raw.description
      : undefined;

  return {
    id,
    title,
    description,
    status: normalizeTaskStatus(raw.status),
  };
};

const normalizeTasks = (raw: unknown): TaskVM[] => {
  if (!Array.isArray(raw)) {
    return [];
  }
  return raw
    .map((item, index) => normalizeTask(item, index))
    .filter((task): task is TaskVM => task !== undefined);
};

const normalizeTodoStatus = (
  status: unknown
): TodoQueueItemVM["status"] => {
  if (
    status === "pending" ||
    status === "in_progress" ||
    status === "blocked" ||
    status === "completed" ||
    status === "failed"
  ) {
    return status;
  }
  return "pending";
};

const normalizeTodoItem = (raw: unknown, index: number): TodoQueueItemVM | undefined => {
  if (!isRecord(raw)) {
    return undefined;
  }

  const id =
    typeof raw.id === "string" && raw.id.trim().length > 0
      ? raw.id.trim()
      : `todo-${index + 1}`;

  const title =
    typeof raw.title === "string" && raw.title.trim().length > 0
      ? raw.title.trim()
      : typeof raw.step === "string" && raw.step.trim().length > 0
        ? raw.step.trim()
        : undefined;

  if (!title) {
    return undefined;
  }

  const description =
    typeof raw.description === "string" && raw.description.trim().length > 0
      ? raw.description.trim()
      : undefined;

  return {
    id,
    title,
    description,
    status: normalizeTodoStatus(raw.status),
  };
};

const parseTodoQueueFromPlan = (planOutput: unknown): TodoQueueVM | undefined => {
  if (!isRecord(planOutput) || !isRecord(planOutput.queue)) {
    return undefined;
  }

  const queue = planOutput.queue;
  if (!Array.isArray(queue.todos)) {
    return undefined;
  }

  const todos = queue.todos
    .map((item, index) => normalizeTodoItem(item, index))
    .filter((todo): todo is TodoQueueItemVM => todo !== undefined);

  if (todos.length === 0) {
    return undefined;
  }

  return {
    todos,
    updatedAt: Date.now(),
  };
};

const deriveTodoQueueFromTasks = (tasks: TaskVM[]): TodoQueueVM | undefined => {
  if (tasks.length === 0) {
    return undefined;
  }

  const todos: TodoQueueItemVM[] = tasks.map((task) => ({
    id: task.id,
    title: task.title,
    description: task.description,
    status: task.status === "completed" ? "completed" : "pending",
  }));

  return {
    todos,
    updatedAt: Date.now(),
  };
};

const isTodoStatus = (status: unknown): status is TodoQueueItemVM["status"] => {
  return VALIDTodoStatuses.includes(status as TodoQueueItemVM["status"]);
};

const normalizeTodoStatus = (status: unknown): TodoQueueItemVM["status"] => {
  if (isTodoStatus(status)) {
    return status;
  }
  return "pending";
};

const deriveTodoQueueFromTasks = (tasks: TaskVM[]): TodoQueueVM => {
  const todos: TodoQueueItemVM[] = tasks.map((task) => ({
    id: task.id,
    title: task.title,
    description: task.description,
    status: task.status === "completed" ? "completed" : "pending",
  }));

  return {
    todos,
    updatedAt: Date.now(),
  };
};

const patchPlanWithTask = (
  existingPlan: PlanVM | undefined,
  rawTask: unknown,
  eventType: string
): PlanVM | undefined => {
  const normalizedTask = normalizeTask(rawTask, 0);
  if (!normalizedTask) {
    return existingPlan;
  }

  const forcedStatus = eventType === "task_completed" ? "completed" : normalizedTask.status;
  const task = {
    ...normalizedTask,
    status: forcedStatus,
  };

  if (!existingPlan) {
    return {
      title: "Execution Plan",
      tasks: [task],
      isStreaming: eventType !== "task_completed",
    };
  }

  const index = existingPlan.tasks.findIndex(
    (existingTask) => existingTask.id === task.id || existingTask.title === task.title
  );

  if (index === -1) {
    return {
      ...existingPlan,
      tasks: [...existingPlan.tasks, task],
      isStreaming: eventType !== "task_completed",
    };
  }

  const nextTasks = [...existingPlan.tasks];
  nextTasks[index] = {
    ...nextTasks[index],
    ...task,
  };
  return {
    ...existingPlan,
    tasks: nextTasks,
    isStreaming: eventType !== "task_completed",
  };
};

const parseStructuredPlanFromEvent = (
  eventType: string,
  event: AgentEventPayload,
  existingPlan: PlanVM | undefined
): PlanVM | undefined => {
  const hasPlanContainer = isRecord(event.plan);
  const hasTasks = Array.isArray(event.tasks);
  if (!hasPlanContainer && !hasTasks && !STRUCTURED_PLAN_EVENT_TYPES.has(eventType)) {
    return undefined;
  }

  const planPayload: Record<string, unknown> | undefined = hasPlanContainer
    ? (event.plan as Record<string, unknown>)
    : undefined;
  const tasks = normalizeTasks(
    hasTasks
      ? event.tasks
      : hasPlanContainer
        ? planPayload?.tasks
        : undefined
  );
  const titleCandidate =
    (hasPlanContainer ? planPayload?.title : event.title) ??
    (hasPlanContainer ? planPayload?.name : event.name);
  const descriptionCandidate =
    (hasPlanContainer ? planPayload?.description : event.description);
  const isStreamingCandidate =
    (hasPlanContainer ? planPayload?.is_streaming : event.is_streaming) ??
    (hasPlanContainer ? planPayload?.isStreaming : event.isStreaming) ??
    (hasPlanContainer ? planPayload?.streaming : event.streaming);

  const title =
    typeof titleCandidate === "string" && titleCandidate.trim().length > 0
      ? titleCandidate
      : existingPlan?.title ?? "Execution Plan";
  const description =
    typeof descriptionCandidate === "string" && descriptionCandidate.trim().length > 0
      ? descriptionCandidate
      : existingPlan?.description;
  const isStreaming =
    typeof isStreamingCandidate === "boolean"
      ? isStreamingCandidate
      : eventType !== "plan_completed";
  const nextTasks = tasks.length > 0 ? tasks : existingPlan?.tasks ?? [];
  if (nextTasks.length === 0) {
    return undefined;
  }

  return {
    title,
    description,
    tasks: nextTasks,
    isStreaming,
  };
};

const parsePlanFromUpdatePlanToolResult = (
  turn: AgentTurnVM,
  event: AgentEventPayload
): PlanVM | undefined => {
  // Extract plan from update_plan tool result in tool_call_completed event
  const eventType = event.type;
  if (eventType !== "tool_call_completed") {
    return undefined;
  }

  const result = event.result;
  if (!isRecord(result)) {
    return undefined;
  }

  const callId = typeof result.call_id === "string" ? result.call_id : "";
  if (!callId || !isRecord(result.output)) {
    return undefined;
  }

  // Find the tool call to verify it's update_plan
  const tool = turn.tools.find((item) => item.callId === callId);
  if (tool?.toolName !== "update_plan") {
    return undefined;
  }

  const output = result.output as Record<string, unknown>;
  if (!isRecord(output.plan)) {
    return undefined;
  }

  // Reuse existing parseStructuredPlanFromEvent with wrapped event
  const wrappedEvent = {
    type: "plan_updated",
    plan: output.plan,
  } as unknown as AgentEventPayload;

  const plan = parseStructuredPlanFromEvent("plan_updated", wrappedEvent, turn.plan);

  // Parse todoQueue from queue.todos if present
  const todoQueue = parseTodoQueueFromPlan(output.plan);

  // Derive todoQueue from plan.tasks if queue.todos not missing
  if (!todoQueue && plan.tasks.length > 0) {
    turn.todoQueue = deriveTodoQueueFromTasks(plan.tasks);
  }

  return plan;
};

const applyStructuredPlanOrTaskEvent = (
  turn: AgentTurnVM,
  eventType: string,
  event: AgentEventPayload
): boolean => {
  const structuredPlan = parseStructuredPlanFromEvent(eventType, event, turn.plan);
  if (structuredPlan) {
    turn.plan = structuredPlan;
    turn.planSource = "structured";
    return true;
  }

  const hasTaskPayload = isRecord(event.task) || STRUCTURED_TASK_EVENT_TYPES.has(eventType);
  if (!hasTaskPayload) {
    return false;
  }

  const taskPayload = isRecord(event.task) ? event.task : event;
  const nextPlan = patchPlanWithTask(turn.plan, taskPayload, eventType);
  if (!nextPlan) {
    return false;
  }

  turn.plan = nextPlan;
  turn.planSource = "structured";
  return true;
};

const parsePlanFromReasoning = (
  reasoningText: string,
  isStreaming: boolean
): PlanVM | undefined => {
  const taskRegex = /^\s*[-*]\s+\[( |x|X)\]\s+(.+)$/;
  const lines = reasoningText.split("\n");
  const tasks: TaskVM[] = [];
  for (const line of lines) {
    const match = line.match(taskRegex);
    if (!match) {
      continue;
    }
    tasks.push({
      id: `task-${tasks.length + 1}`,
      title: match[2].trim(),
      status: match[1].toLowerCase() === "x" ? "completed" : "pending",
    });
  }

  if (tasks.length === 0) {
    return undefined;
  }

  const heading = lines.find((line) => /^#+\s+/.test(line.trim()));
  const description = lines.find(
    (line) => line.trim().length > 0 && !taskRegex.test(line) && !/^#+\s+/.test(line.trim())
  );

  return {
    title: heading ? heading.replace(/^#+\s+/, "").trim() : "Execution Plan",
    description: description?.trim(),
    tasks,
    isStreaming,
  };
};

const upsertToolCall = (
  tools: ToolCallVM[],
  callId: string,
  toolName: string,
  updates: Partial<Omit<ToolCallVM, "callId" | "toolName">>
): ToolCallVM[] => {
  const now = Date.now();
  const index = tools.findIndex((tool) => tool.callId === callId);
  if (index === -1) {
    return [
      ...tools,
      {
        callId,
        toolName,
        state: "input-streaming",
        updatedAt: now,
        ...updates,
      },
    ];
  }

  const next = [...tools];
  next[index] = {
    ...next[index],
    ...updates,
    updatedAt: now,
  };
  return next;
};

const normalizeSubAgentToolStatus = (status: string): string => {
  const normalized = status.toLowerCase();
  if (normalized === "queued" || normalized === "planned" || normalized === "in_progress") {
    return "waiting";
  }
  if (normalized === "running") {
    return "running";
  }
  if (normalized === "completed" || normalized === "done" || normalized === "succeeded") {
    return "completed";
  }
  if (normalized === "failed" || normalized === "error") {
    return "failed";
  }
  return normalized;
};

const upsertSubAgent = (
  subAgents: SubAgentVM[],
  threadId: string,
  updates: Partial<Omit<SubAgentVM, "threadId">>
): SubAgentVM[] => {
  const now = Date.now();
  const index = subAgents.findIndex((subAgent) => subAgent.threadId === threadId);
  if (index === -1) {
    return [
      ...subAgents,
      {
        threadId,
        agentName: "sub-agent",
        status: "running",
        tools: [],
        updatedAt: now,
        ...updates,
      },
    ];
  }

  const next = [...subAgents];
  next[index] = {
    ...next[index],
    ...updates,
    updatedAt: now,
  };
  return next;
};

const deriveTerminalTextFromToolResult = (value: unknown): string | undefined => {
  if (typeof value === "string") {
    return value.trim().length > 0 ? value : undefined;
  }

  if (!isRecord(value)) {
    return undefined;
  }

  const stdout = typeof value.stdout === "string" ? value.stdout : "";
  const stderr = typeof value.stderr === "string" ? value.stderr : "";
  const merged = `${stdout}${stderr}`.trim();
  if (merged.length > 0) {
    return `${stdout}${stderr}`;
  }

  const textualKeys = ["output", "text", "message", "content"] as const;
  for (const key of textualKeys) {
    const candidate = value[key];
    if (typeof candidate === "string" && candidate.trim().length > 0) {
      return candidate;
    }
  }

  return undefined;
};

const upsertQueueItem = (
  queue: QueueVM,
  callId: string,
  toolName: string,
  status: QueueItemVM["status"]
): QueueVM => {
  const now = Date.now();
  const index = queue.items.findIndex((item) => item.callId === callId);
  if (index === -1) {
    return {
      items: [
        ...queue.items,
        {
          callId,
          toolName,
          status,
          updatedAt: now,
        },
      ],
    };
  }

  const next = [...queue.items];
  next[index] = {
    ...next[index],
    status,
    updatedAt: now,
  };
  return {
    items: next,
  };
};

const mapToolCallStatusToToolState = (status: string): ToolState => {
  if (status === "running") {
    return "input-available";
  }
  if (status === "completed") {
    return "output-available";
  }
  if (status === "failed") {
    return "output-error";
  }
  return "input-streaming";
};

const mapToolCallStatusToQueue = (status: string): QueueItemVM["status"] => {
  if (status === "running") {
    return "running";
  }
  if (status === "completed") {
    return "completed";
  }
  if (status === "failed") {
    return "failed";
  }
  return "waiting";
};

const hashSessionColor = (sessionId: string): string => {
  let hash = 0;
  for (let index = 0; index < sessionId.length; index += 1) {
    hash = (hash * 31 + sessionId.charCodeAt(index)) >>> 0;
  }
  return COLORS[hash % COLORS.length];
};

const mapBackendStatus = (status: BackendChatSession["status"]): ChatStatus => {
  if (status === "active") {
    return "thinking";
  }
  if (status === "archived") {
    return "wait-input";
  }
  return "wait-input";
};

const toStoreSession = (session: BackendChatSession): ChatSession => ({
  id: session.id,
  title: session.title,
  color: hashSessionColor(session.id),
  status: mapBackendStatus(session.status),
  createdAt: session.created_at,
  updatedAt: session.updated_at,
});

const toStoreMessage = (message: BackendChatMessage): ChatMessage => ({
  id: message.id,
  sessionId: message.session_id,
  role: message.role,
  content: message.content,
  createdAt: message.created_at,
});

const isBackendMessageId = (id: string): boolean => /^\d+$/.test(id);

const isInFlightSessionStatus = (status: ChatStatus | undefined): boolean =>
  status === "thinking" || status === "tool-call" || status === "outputing";

const mapBackendTurnStatus = (
  status: BackendChatTurnSummary["status"]
): AgentTurnStatus => {
  if (status === "done") {
    return "done";
  }
  if (status === "failed") {
    return "failed";
  }
  if (status === "cancelled") {
    return "cancelled";
  }
  return "started";
};

const toStoreTurn = (turn: BackendChatTurnSummary): AgentTurnVM => ({
  id: turn.id,
  sessionId: turn.session_id,
  createdAt: turn.created_at,
  updatedAt: turn.updated_at,
  status: mapBackendTurnStatus(turn.status),
  assistantText: turn.final_message ?? "",
  reasoning: {
    isStreaming: false,
    isExpanded: false,
    preview: "",
    text: "",
    charCount: 0,
    truncated: false,
    updatedAt: turn.updated_at,
    status: "idle",
  },
  tools: [],
  subAgents: [],
  queue: { items: [] },
  terminal: {
    stdout: "",
    stderr: "",
    output: "",
    isStreaming: false,
    updatedAt: turn.updated_at,
  },
  lastSeq: 0,
});

const mergeTurnWithLocalProcess = (
  incomingTurn: AgentTurnVM,
  existingTurn: AgentTurnVM | undefined
): AgentTurnVM => {
  if (!existingTurn) {
    return incomingTurn;
  }

  const mergedStatus =
    existingTurn.status === "streaming" && incomingTurn.status === "started"
      ? existingTurn.status
      : incomingTurn.status;
  const isFinalStatus =
    mergedStatus === "done" ||
    mergedStatus === "failed" ||
    mergedStatus === "cancelled";
  const reasoningStatus =
    isFinalStatus &&
    (existingTurn.reasoning.status === "started" ||
      existingTurn.reasoning.status === "streaming")
      ? "completed"
      : existingTurn.reasoning.status;

  return {
    ...existingTurn,
    id: incomingTurn.id,
    sessionId: incomingTurn.sessionId,
    createdAt: incomingTurn.createdAt,
    updatedAt: Math.max(existingTurn.updatedAt, incomingTurn.updatedAt),
    status: mergedStatus,
    assistantText:
      incomingTurn.assistantText.trim().length > 0
        ? incomingTurn.assistantText
        : existingTurn.assistantText,
    reasoning: {
      ...existingTurn.reasoning,
      isStreaming: isFinalStatus ? false : existingTurn.reasoning.isStreaming,
      status: reasoningStatus,
      updatedAt: Math.max(existingTurn.reasoning.updatedAt, incomingTurn.updatedAt),
    },
    terminal: {
      ...existingTurn.terminal,
      isStreaming: isFinalStatus ? false : existingTurn.terminal.isStreaming,
      updatedAt: Math.max(existingTurn.terminal.updatedAt, incomingTurn.updatedAt),
    },
    plan: existingTurn.plan
      ? {
          ...existingTurn.plan,
          isStreaming: isFinalStatus ? false : existingTurn.plan.isStreaming,
        }
      : existingTurn.plan,
    error:
      mergedStatus === "failed" || mergedStatus === "cancelled"
        ? existingTurn.error
        : undefined,
  };
};

export const useChatStore = create<ChatState>((set, get) => ({
      sessions: [],
      currentSessionId: null,
      messages: {},
      turns: {},
      turnUiState: {},
      scrollToBottomSignal: {},
      cacheBytes: 0,

      bootstrap: async () => {
        const remoteSessions = await listChatSessions();
        const ensured = remoteSessions.length > 0 ? remoteSessions : [await createChatSessionApi()];
        const sessions = ensured.map(toStoreSession);
        const candidateCurrentId = get().currentSessionId;
        const currentSessionId = candidateCurrentId && sessions.some((s) => s.id === candidateCurrentId)
          ? candidateCurrentId
          : sessions[0]?.id ?? null;

        set((state) => {
          const validIds = new Set(sessions.map((session) => session.id));
          const messages = Object.fromEntries(
            Object.entries(state.messages).filter(([sessionId]) => validIds.has(sessionId))
          );
          const turns = Object.fromEntries(
            Object.entries(state.turns).filter(([sessionId]) => validIds.has(sessionId))
          );
          const turnUiState = Object.fromEntries(
            Object.entries(state.turnUiState).filter(([sessionId]) => validIds.has(sessionId))
          );
          const scrollToBottomSignal = Object.fromEntries(
            Object.entries(state.scrollToBottomSignal).filter(([sessionId]) =>
              validIds.has(sessionId)
            )
          );

          const trimmed = trimChatCacheToBudget(
            sessions,
            messages,
            turns,
            currentSessionId,
            CACHE_BUDGET_BYTES
          );

          return {
            sessions,
            currentSessionId,
            messages: trimmed.messages,
            turns: trimmed.turns,
            turnUiState,
            scrollToBottomSignal,
            cacheBytes: trimmed.estimatedBytes,
          };
        });

        if (currentSessionId) {
          await Promise.all([
            get().loadSessionMessages(currentSessionId, {
              range: "window_24h",
              limit: DEFAULT_LOAD_LIMIT,
            }),
            get().loadSessionTurns(currentSessionId),
          ]);
        }
      },

      createSession: async () => {
        const created = await createChatSessionApi();
        const nextSession = toStoreSession(created);

        set((state) => {
          const sessions = [nextSession, ...state.sessions.filter((session) => session.id !== nextSession.id)];
          const messages = { ...state.messages, [nextSession.id]: state.messages[nextSession.id] ?? [] };
          const turns = { ...state.turns, [nextSession.id]: state.turns[nextSession.id] ?? [] };
          const turnUiState = {
            ...state.turnUiState,
            [nextSession.id]: state.turnUiState[nextSession.id] ?? {},
          };
          const scrollToBottomSignal = {
            ...state.scrollToBottomSignal,
            [nextSession.id]: state.scrollToBottomSignal[nextSession.id] ?? 0,
          };
          const trimmed = trimChatCacheToBudget(
            sessions,
            messages,
            turns,
            nextSession.id,
            CACHE_BUDGET_BYTES
          );

          return {
            sessions,
            currentSessionId: nextSession.id,
            messages: trimmed.messages,
            turns: trimmed.turns,
            turnUiState,
            scrollToBottomSignal,
            cacheBytes: trimmed.estimatedBytes,
          };
        });

        return nextSession.id;
      },

      deleteSession: async (id) => {
        const wasCurrentSession = get().currentSessionId === id;
        await deleteChatSessionApi(id);

        set((state) => {
          const sessions = state.sessions.filter((s) => s.id !== id);
          const messages = { ...state.messages };
          delete messages[id];
          const turns = { ...state.turns };
          delete turns[id];
          const turnUiState = { ...state.turnUiState };
          delete turnUiState[id];
          const scrollToBottomSignal = { ...state.scrollToBottomSignal };
          delete scrollToBottomSignal[id];

          const currentSessionId = state.currentSessionId === id ? (sessions[0]?.id ?? null) : state.currentSessionId;
          const trimmed = trimChatCacheToBudget(
            sessions,
            messages,
            turns,
            currentSessionId,
            CACHE_BUDGET_BYTES
          );

          return {
            sessions,
            messages: trimmed.messages,
            turns: trimmed.turns,
            turnUiState,
            scrollToBottomSignal,
            currentSessionId,
            cacheBytes: trimmed.estimatedBytes,
          };
        });

        if (wasCurrentSession) {
          const nextCurrentSessionId = get().currentSessionId;
          if (nextCurrentSessionId) {
            void get().loadSessionMessages(nextCurrentSessionId, {
              range: "window_24h",
              limit: DEFAULT_LOAD_LIMIT,
            });
            void get().loadSessionTurns(nextCurrentSessionId);
          }
        }
      },

      loadSessionMessages: async (sessionId, options) => {
        const range = options?.range ?? "window_24h";
        const remote = await getChatMessages(sessionId, options);
        const incoming = remote.map(toStoreMessage);
        const isCursorPagination = range === "all" && typeof options?.cursor === "number";

        set((state) => {
          const existing = state.messages[sessionId] ?? [];
          let nextSessionMessages: ChatMessage[];

          if (isCursorPagination) {
            const seen = new Set(existing.map((message) => message.id));
            const older = incoming.filter((message) => !seen.has(message.id));
            nextSessionMessages = [...older, ...existing];
          } else if (range === "all") {
            const seen = new Set<string>();
            nextSessionMessages = [...existing, ...incoming]
              .filter((message) => {
                if (seen.has(message.id)) {
                  return false;
                }
                seen.add(message.id);
                return true;
              })
              .sort((a, b) => a.createdAt - b.createdAt);
          } else {
            const sessionStatus = state.sessions.find((session) => session.id === sessionId)?.status;
            const preservedLocal = isInFlightSessionStatus(sessionStatus)
              ? existing.filter((message) => !isBackendMessageId(message.id))
              : [];
            nextSessionMessages = [...incoming, ...preservedLocal].sort(
              (a, b) => a.createdAt - b.createdAt
            );
          }

          const messages = {
            ...state.messages,
            [sessionId]: nextSessionMessages,
          };
          const trimmed = trimChatCacheToBudget(
            state.sessions,
            messages,
            state.turns,
            state.currentSessionId,
            CACHE_BUDGET_BYTES
          );

          return {
            messages: trimmed.messages,
            turns: trimmed.turns,
            cacheBytes: trimmed.estimatedBytes,
          };
        });
      },

      loadSessionTurns: async (sessionId) => {
        const remote = await getChatTurnSummaries(sessionId);
        const incoming = remote.map(toStoreTurn).sort((a, b) => a.createdAt - b.createdAt);

        set((state) => {
          const existing = state.turns[sessionId] ?? [];
          const existingById = new Map(existing.map((turn) => [turn.id, turn]));
          const mergedIncoming = incoming.map((turn) =>
            mergeTurnWithLocalProcess(turn, existingById.get(turn.id))
          );
          const incomingIds = new Set(mergedIncoming.map((turn) => turn.id));
          const preservedInFlight = existing.filter(
            (turn) =>
              (turn.status === "started" || turn.status === "streaming") &&
              !incomingIds.has(turn.id)
          );
          const nextSessionTurns = [...mergedIncoming, ...preservedInFlight].sort(
            (a, b) => a.createdAt - b.createdAt
          );

          const turns = {
            ...state.turns,
            [sessionId]: nextSessionTurns,
          };
          const trimmed = trimChatCacheToBudget(
            state.sessions,
            state.messages,
            turns,
            state.currentSessionId,
            CACHE_BUDGET_BYTES
          );

          return {
            messages: trimmed.messages,
            turns: trimmed.turns,
            cacheBytes: trimmed.estimatedBytes,
          };
        });
      },

      loadFullSessionMessages: async (sessionId, limit = DEFAULT_LOAD_LIMIT) => {
        const existing = get().messages[sessionId] ?? [];
        const firstMessage = existing[0];
        const parsedCursor = firstMessage ? Number.parseInt(firstMessage.id, 10) : Number.NaN;

        await get().loadSessionMessages(sessionId, {
          range: "all",
          cursor: Number.isFinite(parsedCursor) ? parsedCursor : undefined,
          limit,
        });
      },

      clearSessionCache: (sessionId) => {
        set((state) => {
          const messages = { ...state.messages, [sessionId]: [] };
          const turns = { ...state.turns, [sessionId]: [] };
          const trimmed = trimChatCacheToBudget(
            state.sessions,
            messages,
            turns,
            state.currentSessionId,
            CACHE_BUDGET_BYTES
          );
          return {
            messages: trimmed.messages,
            turns: trimmed.turns,
            cacheBytes: trimmed.estimatedBytes,
          };
        });
      },

      updateSession: async (id, updates) => {
        const nextTitle = updates.title?.trim();
        if (!nextTitle) {
          return;
        }

        const updated = await updateChatSessionApi(id, {
          title: nextTitle,
        });
        const persisted = toStoreSession(updated);

        set((state) => ({
          sessions: state.sessions.map((session) =>
            session.id === id
              ? {
                  ...session,
                  title: persisted.title,
                  updatedAt: persisted.updatedAt,
                }
              : session
          ),
        }));
      },

      setCurrentSession: (id) => {
        set({ currentSessionId: id });
        void get().loadSessionMessages(id, {
          range: "window_24h",
          limit: DEFAULT_LOAD_LIMIT,
        });
        void get().loadSessionTurns(id);
      },

      requestScrollToBottom: (sessionId) => {
        set((state) => ({
          scrollToBottomSignal: {
            ...state.scrollToBottomSignal,
            [sessionId]: (state.scrollToBottomSignal[sessionId] ?? 0) + 1,
          },
        }));
      },

      addMessage: (sessionId, message) => {
        const id = `msg-${Date.now()}-${Math.floor(Math.random() * 1000)}`;
        const newMessage: ChatMessage = {
          ...message,
          id,
          sessionId,
          createdAt: Date.now(),
        };

        set((state) => {
          const messages = {
            ...state.messages,
            [sessionId]: [...(state.messages[sessionId] ?? []), newMessage],
          };
          const trimmed = trimChatCacheToBudget(
            state.sessions,
            messages,
            state.turns,
            state.currentSessionId,
            CACHE_BUDGET_BYTES
          );
          return {
            messages: trimmed.messages,
            turns: trimmed.turns,
            cacheBytes: trimmed.estimatedBytes,
          };
        });

        return id;
      },

      updateSessionStatus: (id, status) => {
        set((state) => ({
          sessions: state.sessions.map((s) =>
            s.id === id ? { ...s, status, updatedAt: Date.now() } : s
          ),
        }));
      },

      ensureAgentTurn: (sessionId, turnId, requestMessageId) => {
        set((state) => {
          const currentTurns = [...(state.turns[sessionId] ?? [])];
          const index = currentTurns.findIndex((turn) => turn.id === turnId);
          if (index !== -1) {
            if (requestMessageId && !currentTurns[index].requestMessageId) {
              currentTurns[index] = {
                ...currentTurns[index],
                requestMessageId,
              };
              const turns = {
                ...state.turns,
                [sessionId]: currentTurns,
              };
              const trimmed = trimChatCacheToBudget(
                state.sessions,
                state.messages,
                turns,
                state.currentSessionId,
                CACHE_BUDGET_BYTES
              );
              return {
                messages: trimmed.messages,
                turns: trimmed.turns,
                cacheBytes: trimmed.estimatedBytes,
              };
            }
            return {};
          }

          currentTurns.push(createEmptyTurn(sessionId, turnId, requestMessageId));
          const turns = {
            ...state.turns,
            [sessionId]: currentTurns,
          };
          const trimmed = trimChatCacheToBudget(
            state.sessions,
            state.messages,
            turns,
            state.currentSessionId,
            CACHE_BUDGET_BYTES
          );
          return {
            messages: trimmed.messages,
            turns: trimmed.turns,
            cacheBytes: trimmed.estimatedBytes,
          };
        });
      },

      setReasoningExpanded: (sessionId, turnId, expanded) => {
        set((state) => {
          const currentTurns = [...(state.turns[sessionId] ?? [])];
          const index = currentTurns.findIndex((turn) => turn.id === turnId);
          if (index === -1) {
            return {};
          }
          const turn = currentTurns[index];
          currentTurns[index] = {
            ...turn,
            reasoning: {
              ...turn.reasoning,
              isExpanded: expanded,
            },
          };
          return {
            turns: {
              ...state.turns,
              [sessionId]: currentTurns,
            },
          };
        });
      },

      setTurnProcessExpanded: (sessionId, turnId, expanded) => {
        set((state) => {
          const sessionUi = { ...(state.turnUiState[sessionId] ?? {}) };
          const turnUi = sessionUi[turnId] ?? createDefaultTurnUiState();

          sessionUi[turnId] = {
            ...turnUi,
            processExpanded: expanded,
          };

          return {
            turnUiState: {
              ...state.turnUiState,
              [sessionId]: sessionUi,
            },
          };
        });
      },

      setTurnSectionExpanded: (sessionId, turnId, section, expanded) => {
        set((state) => {
          const sessionUi = { ...(state.turnUiState[sessionId] ?? {}) };
          const turnUi = sessionUi[turnId] ?? createDefaultTurnUiState();

          sessionUi[turnId] = {
            ...turnUi,
            sectionExpanded: {
              ...turnUi.sectionExpanded,
              [section]: expanded,
            },
          };

          return {
            turnUiState: {
              ...state.turnUiState,
              [sessionId]: sessionUi,
            },
          };
        });
      },

      setTurnCodeExpanded: (sessionId, turnId, codeBlockId, expanded) => {
        set((state) => {
          const sessionUi = { ...(state.turnUiState[sessionId] ?? {}) };
          const turnUi = sessionUi[turnId] ?? createDefaultTurnUiState();
          const codeExpanded = { ...turnUi.codeExpanded };

          if (expanded) {
            codeExpanded[codeBlockId] = true;
          } else {
            delete codeExpanded[codeBlockId];
          }

          sessionUi[turnId] = {
            ...turnUi,
            codeExpanded,
          };

          return {
            turnUiState: {
              ...state.turnUiState,
              [sessionId]: sessionUi,
            },
          };
        });
      },

      restoreToCheckpoint: (sessionId, restoredTurnId, removedTurnIds) => {
        set((state) => {
          const currentTurns = state.turns[sessionId] ?? [];
          const restoredIndex = currentTurns.findIndex(
            (turn) => turn.id === restoredTurnId
          );
          if (restoredIndex === -1) {
            return {};
          }

          const trailingTurnIds = currentTurns
            .slice(restoredIndex + 1)
            .map((turn) => turn.id);
          const removedTurnSet = new Set([...removedTurnIds, ...trailingTurnIds]);
          const keptTurns = currentTurns.filter((turn) => !removedTurnSet.has(turn.id));
          const removedTurns = currentTurns.filter((turn) => removedTurnSet.has(turn.id));
          const removedRequestMessageIds = new Set(
            removedTurns
              .map((turn) => turn.requestMessageId)
              .filter((id): id is string => typeof id === "string" && id.length > 0)
          );
          const cutoffTimestamp = keptTurns.at(-1)?.updatedAt ?? Date.now();
          const currentMessages = state.messages[sessionId] ?? [];
          const keptMessages = currentMessages.filter(
            (message) =>
              !removedRequestMessageIds.has(message.id) &&
              message.createdAt <= cutoffTimestamp
          );

          const nextSessionUi = { ...(state.turnUiState[sessionId] ?? {}) };
          for (const turnId of removedTurnSet) {
            delete nextSessionUi[turnId];
          }

          const messages = {
            ...state.messages,
            [sessionId]: keptMessages,
          };
          const turns = {
            ...state.turns,
            [sessionId]: keptTurns,
          };
          const sessions: ChatSession[] = state.sessions.map((session): ChatSession =>
            session.id === sessionId
              ? {
                  ...session,
                  status: "wait-input",
                  updatedAt: Date.now(),
                }
              : session
          );
          const trimmed = trimChatCacheToBudget(
            sessions,
            messages,
            turns,
            state.currentSessionId,
            CACHE_BUDGET_BYTES
          );

          return {
            sessions,
            messages: trimmed.messages,
            turns: trimmed.turns,
            turnUiState: {
              ...state.turnUiState,
              [sessionId]: nextSessionUi,
            },
            cacheBytes: trimmed.estimatedBytes,
          };
        });
      },

      applyAgentStreamEnvelope: (envelope) => {
        set((state) => {
          const sessionId = envelope.sessionId;
          const turnId = envelope.turnId;
          const source = envelope.source; // "run" or "ui"
          const event = envelope.event ?? { type: "unknown" };
          const eventType = String(event.type ?? "");

          const currentTurns = [...(state.turns[sessionId] ?? [])];
          let index = currentTurns.findIndex((turn) => turn.id === turnId);
          if (index === -1) {
            currentTurns.push(createEmptyTurn(sessionId, turnId));
            index = currentTurns.length - 1;
          }

          const existingTurn = currentTurns[index];
          if (envelope.seq <= existingTurn.lastSeq) {
            return {};
          }

          const now = Date.now();
          const turn: AgentTurnVM = {
            ...existingTurn,
            updatedAt: now,
            lastSeq: envelope.seq,
            reasoning: {
              ...existingTurn.reasoning,
            },
            queue: {
              items: [...existingTurn.queue.items],
            },
            terminal: {
              ...existingTurn.terminal,
            },
            tools: [...existingTurn.tools],
            subAgents: [...existingTurn.subAgents],
          };

          const updateSessionStatus = (status: ChatStatus) => {
            const sessions = state.sessions.map((session) =>
              session.id === sessionId
                ? { ...session, status, updatedAt: now }
                : session
            );
            return sessions;
          };

          let sessions = state.sessions;
          if (eventType === "turn_start") {
            turn.status = "started";
            sessions = updateSessionStatus("thinking");
          }
          if (eventType === "message_delta") {
            const delta = String(event.delta ?? "");
            turn.assistantText += delta;
            turn.status = "streaming";
            sessions = updateSessionStatus("outputing");
          }
          if (eventType === "reasoning_started") {
            turn.reasoning.isStreaming = true;
            turn.reasoning.status = "started";
            turn.reasoning.updatedAt = now;
            sessions = updateSessionStatus("thinking");
          }
          if (eventType === "reasoning_delta") {
            const incoming = String(event.delta ?? "");
            // Use code point count for consistent handling of emoji/CJK characters
            // This aligns with backend char_count semantics
            const incomingCodePoints = Array.from(incoming);
            if (incomingCodePoints.length > 0) {
              turn.reasoning.charCount += incomingCodePoints.length;

              if (!turn.reasoning.truncated) {
                const currentCodePoints = Array.from(turn.reasoning.text).length;
                const remaining = Math.max(REASONING_CHAR_LIMIT - currentCodePoints, 0);
                const visibleChars = incomingCodePoints.slice(0, remaining);
                const visible = visibleChars.join("");
                turn.reasoning.text += visible;
                turn.reasoning.truncated = visibleChars.length < incomingCodePoints.length;
                turn.reasoning.preview = toPreview(turn.reasoning.text);
              }

              turn.reasoning.isStreaming = true;
              turn.reasoning.status = "streaming";
              turn.reasoning.updatedAt = now;
              if (turn.planSource !== "structured") {
                turn.plan = parsePlanFromReasoning(turn.reasoning.text, true);
                if (turn.plan) {
                  turn.planSource = "reasoning-fallback";
                }
              }
              sessions = updateSessionStatus("thinking");
            }
          }
          if (eventType === "reasoning_completed") {
            turn.reasoning.isStreaming = false;
            turn.reasoning.status = turn.reasoning.status === "error" ? "error" : "completed";
            if (typeof event.char_count === "number") {
              turn.reasoning.charCount = event.char_count;
            }
            if (typeof event.truncated === "boolean") {
              turn.reasoning.truncated = event.truncated;
            }
            turn.reasoning.updatedAt = now;
            if (turn.planSource !== "structured") {
              turn.plan = parsePlanFromReasoning(turn.reasoning.text, false);
              if (turn.plan) {
                turn.planSource = "reasoning-fallback";
              }
            }
          }
          if (eventType === "tool_call_requested") {
            const callId = String(event.call_id ?? "");
            const toolName = String(event.tool_name ?? "tool");
            const input =
              typeof event.arguments === "object" && event.arguments !== null
                ? (event.arguments as Record<string, unknown>)
                : undefined;
            turn.tools = upsertToolCall(turn.tools, callId, toolName, {
              state: "input-streaming",
              input,
            });
            turn.queue = upsertQueueItem(turn.queue, callId, toolName, "waiting");
            sessions = updateSessionStatus("tool-call");
          }
          if (eventType === "tool_queued") {
            const callId = String(event.call_id ?? "");
            const toolName = String(event.tool_name ?? "tool");
            turn.queue = upsertQueueItem(turn.queue, callId, toolName, "waiting");
            sessions = updateSessionStatus("tool-call");
          }
          if (eventType === "tool_dequeued") {
            const callId = String(event.call_id ?? "");
            const toolName = String(event.tool_name ?? "tool");
            turn.queue = upsertQueueItem(turn.queue, callId, toolName, "running");
            sessions = updateSessionStatus("tool-call");
          }
          if (eventType === "tool_call_progress") {
            const callId = String(event.call_id ?? "");
            const status = String(event.status ?? "input-streaming");
            const found = turn.tools.find((tool) => tool.callId === callId);
            const toolName = found?.toolName ?? "tool";
            turn.tools = upsertToolCall(turn.tools, callId, toolName, {
              state: mapToolCallStatusToToolState(status),
            });
            turn.queue = upsertQueueItem(
              turn.queue,
              callId,
              toolName,
              mapToolCallStatusToQueue(status)
            );
            sessions = updateSessionStatus("tool-call");
          }
          // Only process terminal events from "ui" source to avoid duplicate consumption
          // Run events and ui events may both emit stdout/stderr, we only consume ui
          if (eventType === "tool_stdout_delta" && source === "ui") {
            const delta = String(event.delta ?? "");
            turn.terminal.stdout += delta;
            turn.terminal.output = `${turn.terminal.stdout}${turn.terminal.stderr}`;
            turn.terminal.isStreaming = true;
            turn.terminal.updatedAt = now;
          }
          if (eventType === "tool_stderr_delta" && source === "ui") {
            const delta = String(event.delta ?? "");
            turn.terminal.stderr += delta;
            turn.terminal.output = `${turn.terminal.stdout}${turn.terminal.stderr}`;
            turn.terminal.isStreaming = true;
            turn.terminal.updatedAt = now;
          }
          if (eventType === "tool_exit" && source === "ui") {
            turn.terminal.isStreaming = false;
            if (typeof event.exit_code === "number") {
              turn.terminal.exitCode = event.exit_code;
            }
            if (typeof event.duration_ms === "number") {
              turn.terminal.durationMs = event.duration_ms;
            }
            turn.terminal.updatedAt = now;
          }
          if (eventType === "tool_call_completed") {
            const result = (event.result ?? {}) as {
              call_id?: string;
              output?: unknown;
              is_error?: boolean;
            };
            const callId = String(result.call_id ?? "");
            const found = turn.tools.find((tool) => tool.callId === callId);
            const toolName = found?.toolName ?? "tool";
            const isError = Boolean(result.is_error);
            let errorText: string | undefined;
            if (isError) {
              const output = result.output as Record<string, unknown> | undefined;
              if (output && typeof output.error === "string") {
                errorText = output.error;
              }
            }
            turn.tools = upsertToolCall(turn.tools, callId, toolName, {
              state: isError ? "output-error" : "output-available",
              output: result.output,
              errorText,
            });
            turn.queue = upsertQueueItem(
              turn.queue,
              callId,
              toolName,
              isError ? "failed" : "completed"
            );
            if (!turn.terminal.output && result.output !== undefined) {
              const terminalText = deriveTerminalTextFromToolResult(result.output);
              if (terminalText) {
                turn.terminal.output = terminalText;
              }
            }
            if (!turn.terminal.output && isError && errorText) {
              turn.terminal.output = errorText;
            }
            turn.terminal.updatedAt = now;
          }
          if (eventType === "sub_agent_updated") {
            const threadId = String(event.thread_id ?? "");
            if (threadId.length > 0) {
              const status = String(event.status ?? "running").toLowerCase();
              const agentName = String(event.agent_name ?? "sub-agent");
              const errorText =
                typeof event.error === "string" && event.error.trim().length > 0
                  ? event.error
                  : undefined;
              const activeTools = Array.isArray(event.active_tools)
                ? event.active_tools
                    .map((item) => {
                      if (
                        typeof item !== "object" ||
                        item === null ||
                        !("call_id" in item) ||
                        !("tool_name" in item)
                      ) {
                        return null;
                      }
                      const callId = String((item as Record<string, unknown>).call_id ?? "");
                      const toolName = String(
                        (item as Record<string, unknown>).tool_name ?? "tool"
                      );
                      if (!callId) {
                        return null;
                      }
                      return {
                        callId,
                        toolName,
                        status: normalizeSubAgentToolStatus(
                          String((item as Record<string, unknown>).status ?? "running")
                        ),
                        updatedAt: now,
                      };
                    })
                    .filter((item): item is SubAgentToolVM => item !== null)
                : [];

              turn.subAgents = upsertSubAgent(turn.subAgents, threadId, {
                agentName,
                status,
                tools: activeTools,
                errorText,
              });

              if (
                status === "running" ||
                status === "pending" ||
                status === "waiting"
              ) {
                sessions = updateSessionStatus("tool-call");
              }
            }
          }
          // Extract structured plan from update_plan tool result
          const toolPlan = parsePlanFromUpdatePlanToolResult(turn, event);
          if (toolPlan) {
            turn.plan = toolPlan;
            turn.planSource = "structured";
          }
          applyStructuredPlanOrTaskEvent(turn, eventType, event);
          if (eventType === "done" || eventType === "turn_done") {
            if (typeof event.summary === "string" && !turn.assistantText) {
              turn.assistantText = event.summary;
            }
            if (typeof event.final_message === "string" && !turn.assistantText) {
              turn.assistantText = event.final_message;
            }
            if (!turn.assistantText) {
              const latestToolText = [...turn.tools]
                .reverse()
                .map((tool) => tool.output)
                .find(
                  (output): output is string =>
                    typeof output === "string" &&
                    output.trim().length > 0
                );
              if (latestToolText) {
                turn.assistantText = latestToolText;
              } else if (turn.terminal.output.trim().length > 0) {
                turn.assistantText = turn.terminal.output;
              }
            }
            turn.status = "done";
            turn.reasoning.isStreaming = false;
            if (
              turn.reasoning.status === "started" ||
              turn.reasoning.status === "streaming"
            ) {
              turn.reasoning.status = "completed";
            }
            if (turn.planSource === "structured") {
              turn.plan = turn.plan ? { ...turn.plan, isStreaming: false } : turn.plan;
            } else {
              turn.plan = parsePlanFromReasoning(turn.reasoning.text, false);
              if (turn.plan) {
                turn.planSource = "reasoning-fallback";
              }
            }
            sessions = updateSessionStatus("await-input");
          }
          if (eventType === "error" || eventType === "turn_failed") {
            const message = String(event.message ?? "turn failed");
            turn.error = message;
            turn.status = Boolean(event.cancelled) ? "cancelled" : "failed";
            turn.reasoning.isStreaming = false;
            turn.reasoning.status = "error";
            turn.reasoning.updatedAt = now;
            if (turn.plan) {
              turn.plan = {
                ...turn.plan,
                isStreaming: false,
              };
            }
            sessions = updateSessionStatus("wait-input");
          }

          currentTurns[index] = turn;
          const turns = {
            ...state.turns,
            [sessionId]: currentTurns,
          };
          const trimmed = trimChatCacheToBudget(
            sessions,
            state.messages,
            turns,
            state.currentSessionId,
            CACHE_BUDGET_BYTES
          );

          return {
            sessions,
            messages: trimmed.messages,
            turns: trimmed.turns,
            cacheBytes: trimmed.estimatedBytes,
          };
        });
      },
    }));
