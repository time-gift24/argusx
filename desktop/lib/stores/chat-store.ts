import type { AgentEventPayload, AgentStreamEnvelope } from "@/lib/api/chat";
import type { ToolState } from "@/types";

import { create } from "zustand";
import { persist } from "zustand/middleware";

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

export type AgentTurnStatus =
  | "started"
  | "streaming"
  | "done"
  | "failed"
  | "cancelled";

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
  queue: QueueVM;
  terminal: TerminalVM;
  plan?: PlanVM;
  planSource?: "structured" | "reasoning-fallback";
  error?: string;
  lastSeq: number;
}

interface ChatState {
  sessions: ChatSession[];
  currentSessionId: string | null;
  messages: Record<string, ChatMessage[]>;
  turns: Record<string, AgentTurnVM[]>;

  createSession: () => string;
  deleteSession: (id: string) => void;
  updateSession: (id: string, updates: Partial<Pick<ChatSession, "title" | "color">>) => void;
  setCurrentSession: (id: string) => void;
  addMessage: (
    sessionId: string,
    message: Omit<ChatMessage, "id" | "sessionId" | "createdAt">
  ) => string;
  updateSessionStatus: (id: string, status: ChatStatus) => void;
  ensureAgentTurn: (sessionId: string, turnId: string, requestMessageId?: string) => void;
  setReasoningExpanded: (sessionId: string, turnId: string, expanded: boolean) => void;
  applyAgentStreamEnvelope: (envelope: AgentStreamEnvelope) => void;
}

const COLORS = ["chart-1", "chart-2", "chart-3", "chart-4", "chart-5"];
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

export const useChatStore = create<ChatState>()(
  persist(
    (set, get) => ({
      sessions: [],
      currentSessionId: null,
      messages: {},
      turns: {},

      createSession: () => {
        const id = `session-${Date.now()}`;
        const now = Date.now();
        const colorIndex = get().sessions.length % COLORS.length;

        const newSession: ChatSession = {
          id,
          title: `Chat ${get().sessions.length + 1}`,
          color: COLORS[colorIndex],
          status: "wait-input",
          createdAt: now,
          updatedAt: now,
        };

        set((state) => ({
          sessions: [...state.sessions, newSession],
          currentSessionId: id,
          messages: { ...state.messages, [id]: [] },
          turns: { ...state.turns, [id]: [] },
        }));

        return id;
      },

      deleteSession: (id) => {
        set((state) => {
          const sessions = state.sessions.filter((s) => s.id !== id);
          const messages = { ...state.messages };
          delete messages[id];
          const turns = { ...state.turns };
          delete turns[id];

          let currentSessionId = state.currentSessionId;
          if (currentSessionId === id) {
            currentSessionId = sessions[0]?.id ?? null;
          }

          return { sessions, messages, turns, currentSessionId };
        });
      },

      updateSession: (id, updates) => {
        set((state) => ({
          sessions: state.sessions.map((s) =>
            s.id === id ? { ...s, ...updates, updatedAt: Date.now() } : s
          ),
        }));
      },

      setCurrentSession: (id) => {
        set({ currentSessionId: id });
      },

      addMessage: (sessionId, message) => {
        const id = `msg-${Date.now()}-${Math.floor(Math.random() * 1000)}`;
        const newMessage: ChatMessage = {
          ...message,
          id,
          sessionId,
          createdAt: Date.now(),
        };

        set((state) => ({
          messages: {
            ...state.messages,
            [sessionId]: [...(state.messages[sessionId] ?? []), newMessage],
          },
        }));

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
              return {
                turns: {
                  ...state.turns,
                  [sessionId]: currentTurns,
                },
              };
            }
            return {};
          }

          currentTurns.push(createEmptyTurn(sessionId, turnId, requestMessageId));
          return {
            turns: {
              ...state.turns,
              [sessionId]: currentTurns,
            },
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
              const output =
                typeof result.output === "string"
                  ? result.output
                  : JSON.stringify(result.output, null, 2);
              turn.terminal.output = output;
            }
            turn.terminal.updatedAt = now;
          }
          applyStructuredPlanOrTaskEvent(turn, eventType, event);
          if (eventType === "done" || eventType === "turn_done") {
            if (typeof event.summary === "string" && !turn.assistantText) {
              turn.assistantText = event.summary;
            }
            if (typeof event.final_message === "string" && !turn.assistantText) {
              turn.assistantText = event.final_message;
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

          return {
            sessions,
            turns: {
              ...state.turns,
              [sessionId]: currentTurns,
            },
          };
        });
      },
    }),
    {
      name: "chat-storage",
      version: 4,
      migrate: (persistedState: unknown) => {
        const state = (persistedState ?? {}) as Partial<ChatState>;
        // Reset transient streaming states after hydration to avoid stale "streaming" state
        // This fixes the issue where refreshing the page leaves the UI in a streaming state
        const turns = state.turns ?? {};
        const resetTurns: typeof turns = {};
        for (const sessionId in turns) {
          resetTurns[sessionId] = (turns[sessionId] ?? []).map((turn) => {
            const legacyReasoningStatus = String(
              (turn.reasoning as { status?: unknown }).status ?? "idle"
            );
            const nextReasoningStatus: ReasoningVM["status"] =
              legacyReasoningStatus === "thinking"
                ? "streaming"
                : legacyReasoningStatus === "done"
                  ? "completed"
                  : legacyReasoningStatus === "started" ||
                      legacyReasoningStatus === "streaming" ||
                      legacyReasoningStatus === "completed" ||
                      legacyReasoningStatus === "error"
                    ? legacyReasoningStatus
                    : "idle";

            return {
              ...turn,
              reasoning: {
                ...turn.reasoning,
                isStreaming: false,
                status: nextReasoningStatus,
              },
              queue: {
                ...turn.queue,
                items: (turn.queue?.items ?? []).map((item) => {
                  const legacyQueueStatus = String(
                    (item as { status?: unknown }).status ?? "waiting"
                  );
                  const nextQueueStatus: QueueItemVM["status"] =
                    legacyQueueStatus === "queued" ||
                    legacyQueueStatus === "waiting"
                      ? "waiting"
                      : legacyQueueStatus === "running" ||
                          legacyQueueStatus === "completed" ||
                          legacyQueueStatus === "failed"
                        ? legacyQueueStatus
                        : "waiting";
                  return {
                    ...item,
                    status: nextQueueStatus,
                  };
                }),
              },
              terminal: {
                ...turn.terminal,
                isStreaming: false,
              },
              status: turn.status === "streaming" ? "done" : turn.status,
              plan:
                turn.plan !== undefined
                  ? {
                      ...turn.plan,
                      isStreaming: false,
                    }
                  : undefined,
              planSource: turn.plan ? "reasoning-fallback" : undefined,
            };
          });
        }
        // Also reset session statuses to "wait-input" to avoid stale streaming states
        const sessions = (state.sessions ?? []).map((session) => ({
          ...session,
          status: session.status === "thinking" || session.status === "tool-call" || session.status === "outputing"
            ? "wait-input"
            : session.status,
        }));
        return {
          ...state,
          turns: resetTurns,
          sessions,
        };
      },
    }
  )
);
