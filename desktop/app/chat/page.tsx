"use client";

import {
  startTransition,
  type MutableRefObject,
  useEffect,
  useEffectEvent,
  useRef,
  useState,
} from "react";
import { Streamdown } from "streamdown";

import {
  FloatingPlanCard,
  PromptComposer,
  ReadTaskGroup,
  Reasoning,
  ToolPermissionConfirmation,
  ToolCallItem,
  type PlanSnapshot,
  type PromptComposerSubmitPayload,
} from "@/components/ai";
import { Checkpoint } from "@/components/ai-elements/checkpoint";
import { Separator } from "@/components/ui/separator";
import {
  useTurn,
  type DesktopTurnEvent,
  type HydratedChatTurn,
  type PermissionDecision,
} from "@/lib/chat";

const AGENTS = [
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

type ToolCallStatus =
  | "running"
  | "success"
  | "failed"
  | "timed_out"
  | "denied"
  | "cancelled";

type TurnStatus = "running" | "completed" | "cancelled" | "failed";

type ToolCallView = {
  callId: string;
  name: string;
  argumentsJson: string;
  outputSummary?: string;
  errorSummary?: string;
  status: ToolCallStatus;
};

type PendingPermissionView = {
  requestId: string;
  toolCallId: string;
};

type ResolvedPermissionView = PendingPermissionView & {
  decision: PermissionDecision;
};

type ChatTurnView = {
  clientKey: string;
  assistantText: string;
  error: string | null;
  lastResolvedPermission: ResolvedPermissionView | null;
  latestPlan: PlanSnapshot | null;
  pendingPermissions: PendingPermissionView[];
  prompt: string;
  reasoningText: string;
  status: TurnStatus;
  toolCalls: ToolCallView[];
  turnId: string | null;
};

const AUTO_SCROLL_THRESHOLD_PX = 96;
const COMPOSER_SAFE_GAP_PX = 24;
const MIN_COMPOSER_OFFSET_PX = 220;
export const PERMISSION_RESOLUTION_FEEDBACK_MS = 1800;

export default function ChatPage() {
  const {
    cancelTurn,
    loadActiveChatThread,
    resolveTurnPermission,
    startTurn,
    subscribe,
  } = useTurn();
  const [composerOffset, setComposerOffset] = useState(MIN_COMPOSER_OFFSET_PX);
  const [permissionActionError, setPermissionActionError] = useState<string | null>(null);
  const [permissionActionKey, setPermissionActionKey] = useState<string | null>(null);
  const [turns, setTurns] = useState<ChatTurnView[]>([]);
  const composerShellRef = useRef<HTMLDivElement>(null);
  const scrollViewportRef = useRef<HTMLDivElement>(null);
  const shouldAutoScrollRef = useRef(true);
  const turnSequenceRef = useRef(0);
  const turnsRef = useRef<ChatTurnView[]>([]);

  useEffect(() => {
    turnsRef.current = turns;
  }, [turns]);

  const handleTurnEvent = useEffectEvent((event: DesktopTurnEvent) => {
    startTransition(() => {
      setTurns((current) => reduceTurnEvent(current, event));
    });
  });

  useEffect(() => {
    let cancelled = false;

    void loadActiveChatThread()
      .then((hydratedTurns) => {
        if (cancelled) {
          return;
        }

        startTransition(() => {
          setTurns((current) =>
            current.length === 0 ? hydratedTurns.map(hydrateChatTurn) : current
          );
        });
      })
      .catch((error) => {
        console.error("Failed to hydrate active chat thread", error);
      });

    return () => {
      cancelled = true;
    };
  }, [loadActiveChatThread]);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    let cancelled = false;

    void subscribe((event) => {
      handleTurnEvent(event);
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }

      dispose = unlisten;
    });

    return () => {
      cancelled = true;
      dispose?.();
    };
  }, [subscribe]);

  const syncComposerOffset = useEffectEvent(() => {
    const nextHeight = composerShellRef.current?.offsetHeight ?? 0;
    setComposerOffset(
      Math.max(MIN_COMPOSER_OFFSET_PX, nextHeight + COMPOSER_SAFE_GAP_PX)
    );
  });

  useEffect(() => {
    syncComposerOffset();

    if (
      typeof ResizeObserver === "undefined" ||
      !composerShellRef.current
    ) {
      return;
    }

    const observer = new ResizeObserver(() => {
      syncComposerOffset();
    });

    observer.observe(composerShellRef.current);

    return () => {
      observer.disconnect();
    };
  }, [syncComposerOffset]);

  useEffect(() => {
    const viewport = scrollViewportRef.current;
    if (!viewport || !shouldAutoScrollRef.current) {
      return;
    }

    viewport.scrollTop = viewport.scrollHeight;
  }, [composerOffset, turns]);

  const handleScroll = useEffectEvent(() => {
    const viewport = scrollViewportRef.current;
    if (!viewport) {
      return;
    }

    shouldAutoScrollRef.current = isNearBottom(viewport);
  });

  const visiblePermission = selectVisiblePermission(turns);
  const visiblePermissionKey = visiblePermission
    ? `${visiblePermission.turnId}:${visiblePermission.requestId}:${visiblePermission.state}`
    : null;
  const activeFloatingPlan = selectActiveFloatingPlan(turns);
  const latestResolvedPermission = findLatestResolvedPermission(turns);
  const latestResolvedPermissionKey = latestResolvedPermission
    ? `${latestResolvedPermission.turnId}:${latestResolvedPermission.requestId}`
    : null;

  useEffect(() => {
    setPermissionActionError(null);
    setPermissionActionKey(null);
  }, [visiblePermissionKey]);

  useEffect(() => {
    if (!latestResolvedPermission) {
      return;
    }

    const timeout = window.setTimeout(() => {
      startTransition(() => {
        setTurns((current) =>
          clearResolvedPermission(
            current,
            latestResolvedPermission.turnId,
            latestResolvedPermission.requestId
          )
        );
      });
    }, PERMISSION_RESOLUTION_FEEDBACK_MS);

    return () => {
      window.clearTimeout(timeout);
    };
  }, [latestResolvedPermissionKey]);

  const handlePermissionDecision = async (
    turnId: string,
    requestId: string,
    decision: PermissionDecision
  ) => {
    setPermissionActionError(null);
    setPermissionActionKey(requestId);

    try {
      await resolveTurnPermission(turnId, requestId, decision);
    } catch (error) {
      setPermissionActionError(
        error instanceof Error ? error.message : "Unable to resolve permission."
      );
      setPermissionActionKey(null);
    }
  };

  const handleSubmit = async (payload: PromptComposerSubmitPayload) => {
    if (payload.category !== "agent") {
      throw new Error("Workflow turns are not implemented yet.");
    }

    const clientKey = createClientTurnKey(turnSequenceRef);
    const runningTurn = getRunningTurn(turnsRef.current);

    startTransition(() => {
      setTurns((current) => [...current, createPendingTurn(clientKey, payload.draft)]);
    });

    try {
      if (runningTurn?.turnId) {
        await cancelTurn(runningTurn.turnId);
      }

      const result = await startTurn({
        prompt: payload.draft,
        targetId: payload.selectionId,
        targetKind: "agent",
      });

      startTransition(() => {
        setTurns((current) => assignTurnId(current, clientKey, result.turnId));
      });
    } catch (error) {
      startTransition(() => {
        setTurns((current) =>
          markTurnFailed(
            current,
            clientKey,
            error instanceof Error ? error.message : "Unable to start turn."
          )
        );
      });
      throw error;
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col p-4 lg:p-6">
      <div className="relative mx-auto flex min-h-0 w-full max-w-5xl flex-1 flex-col">
        <div
          className="scrollbar-hide min-h-0 flex-1 overflow-y-auto"
          data-slot="chat-scroll-viewport"
          onScroll={handleScroll}
          ref={scrollViewportRef}
        >
          <div
            className="flex min-h-full flex-col justify-end"
            data-slot="chat-scroll-content"
            style={{ paddingBottom: `${composerOffset}px` }}
          >
            {turns.length > 0 ? (
              <div className="flex flex-col gap-8 py-4">
                {turns.map((turn, index) => (
                  <section
                    className="space-y-4"
                    data-slot="chat-turn"
                    key={turn.clientKey}
                  >
                    <div className="flex items-center gap-3">
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-3 text-muted-foreground">
                          <Separator className="flex-1" />
                          <Checkpoint className="shrink-0 gap-3 text-muted-foreground">
                            <span className="shrink-0 text-[11px] font-medium tracking-[0.08em]">
                              {`第 ${index + 1} 轮`}
                            </span>
                          </Checkpoint>
                        </div>
                      </div>
                      {turn.status === "running" && turn.turnId ? (
                        <button
                          className="shrink-0 rounded-full border border-border/60 px-3 py-1 text-xs text-foreground transition-colors hover:bg-muted/60"
                          onClick={() => {
                            if (turn.turnId) {
                              void cancelTurn(turn.turnId);
                            }
                          }}
                          type="button"
                        >
                          Cancel
                        </button>
                      ) : null}
                    </div>

                    <div className="flex">
                      <div
                        className="ml-auto max-w-[min(32rem,80%)] rounded-3xl bg-muted px-4 py-3 text-sm text-foreground shadow-sm"
                        data-slot="chat-turn-user"
                      >
                        {turn.prompt}
                      </div>
                    </div>

                    <div
                      className="space-y-4 text-sm"
                      data-slot="chat-turn-assistant"
                    >
                      {turn.assistantText ? (
                        <Streamdown isAnimating={turn.status === "running"}>
                          {turn.assistantText}
                        </Streamdown>
                      ) : turn.status === "running" ? (
                        <p className="text-sm text-muted-foreground">
                          Waiting for model output...
                        </p>
                      ) : null}

                      {turn.reasoningText ? (
                        <Reasoning
                          isRunning={turn.status === "running"}
                          runKey={turn.turnId ?? turn.clientKey}
                        >
                          {turn.reasoningText}
                        </Reasoning>
                      ) : null}

                      {getRecentReadTasks(turn).length > 0 ? (
                        <ReadTaskGroup items={getRecentReadTasks(turn)} />
                      ) : null}

                      {turn.toolCalls
                        .filter(
                          (toolCall) =>
                            toolCall.name !== "update_plan" &&
                            !isReadToolName(toolCall.name)
                        )
                        .map((toolCall) => (
                        <ToolCallItem
                          errorSummary={toolCall.errorSummary}
                          inputSummary={formatArgumentsSummary(toolCall.argumentsJson)}
                          isRunning={toolCall.status === "running"}
                          key={toolCall.callId}
                          name={toolCall.name}
                          outputSummary={toolCall.outputSummary}
                          runKey={turn.turnId ?? turn.clientKey}
                        />
                        ))}

                      {turn.error ? (
                        <p className="text-sm text-destructive" role="alert">
                          {turn.error}
                        </p>
                      ) : null}
                    </div>
                  </section>
                ))}
              </div>
            ) : (
              <div className="flex min-h-full items-end pb-4">
                <p className="text-sm text-muted-foreground">
                  Start a turn to stream assistant output, reasoning, and tool activity.
                </p>
              </div>
            )}
          </div>
        </div>
        <div
          className="pointer-events-none absolute bottom-4 left-1/2 z-10 w-full max-w-3xl -translate-x-1/2 px-4 lg:px-0"
          data-slot="chat-composer-shell"
          ref={composerShellRef}
        >
          {visiblePermission ? (
            <ToolPermissionConfirmation
              argumentsSummary={visiblePermission.argumentsSummary}
              className="pointer-events-auto absolute bottom-[calc(100%+0.75rem)] left-1/2 w-full -translate-x-1/2"
              error={
                visiblePermission.state === "requested"
                  ? permissionActionError
                  : null
              }
              isSubmitting={
                visiblePermission.state === "requested" &&
                permissionActionKey === visiblePermission.requestId
              }
              onAllow={
                visiblePermission.state === "requested"
                  ? () =>
                      void handlePermissionDecision(
                        visiblePermission.turnId,
                        visiblePermission.requestId,
                        "allow"
                      )
                  : undefined
              }
              onDeny={
                visiblePermission.state === "requested"
                  ? () =>
                      void handlePermissionDecision(
                        visiblePermission.turnId,
                        visiblePermission.requestId,
                        "deny"
                      )
                  : undefined
              }
              requestId={visiblePermission.requestId}
              state={visiblePermission.state}
              toolName={visiblePermission.toolName}
            />
          ) : null}
          {activeFloatingPlan ? (
            <FloatingPlanCard plan={activeFloatingPlan} />
          ) : null}
          <div className="pointer-events-auto">
            <PromptComposer
              agents={[...AGENTS]}
              onSubmit={handleSubmit}
              workflows={[]}
            />
          </div>
        </div>
      </div>
    </div>
  );
}

function createClientTurnKey(sequenceRef: MutableRefObject<number>) {
  sequenceRef.current += 1;
  return `client-turn-${sequenceRef.current}`;
}

function hydrateChatTurn(turn: HydratedChatTurn): ChatTurnView {
  return {
    assistantText: turn.assistantText,
    clientKey: turn.turnId,
    error: turn.error,
    lastResolvedPermission: null,
    latestPlan: turn.latestPlan ? parsePlanSnapshot(turn.latestPlan) : null,
    pendingPermissions: [],
    prompt: turn.prompt,
    reasoningText: turn.reasoningText,
    status: turn.status,
    toolCalls: turn.toolCalls.map((toolCall) => ({
      argumentsJson: toolCall.argumentsJson,
      callId: toolCall.callId,
      errorSummary: toolCall.errorSummary ?? undefined,
      name: toolCall.name,
      outputSummary: toolCall.outputSummary ?? undefined,
      status: toolCall.status,
    })),
    turnId: turn.turnId,
  };
}

export function createPendingTurn(clientKey: string, prompt: string): ChatTurnView {
  return {
    assistantText: "",
    clientKey,
    error: null,
    lastResolvedPermission: null,
    latestPlan: null,
    pendingPermissions: [],
    prompt,
    reasoningText: "",
    status: "running",
    toolCalls: [],
    turnId: null,
  };
}

function assignTurnId(
  turns: ChatTurnView[],
  clientKey: string,
  turnId: string
): ChatTurnView[] {
  return turns.map((turn) =>
    turn.clientKey === clientKey && turn.turnId === null
      ? {
          ...turn,
          turnId,
        }
      : turn
  );
}

function markTurnFailed(
  turns: ChatTurnView[],
  clientKey: string,
  error: string
): ChatTurnView[] {
  return turns.map((turn) =>
    turn.clientKey === clientKey
      ? {
          ...turn,
          error,
          status: "failed",
        }
      : turn
  );
}

function getRunningTurn(turns: ChatTurnView[]) {
  for (let index = turns.length - 1; index >= 0; index -= 1) {
    if (turns[index].status === "running") {
      return turns[index];
    }
  }

  return null;
}

function reduceTurnEvent(
  current: ChatTurnView[],
  event: DesktopTurnEvent
): ChatTurnView[] {
  const targetIndex = findTurnIndex(current, event.turnId);

  if (targetIndex === -1) {
    return current;
  }

  const nextTurn = reduceTurnEventForTurn(current[targetIndex], event);
  if (nextTurn === current[targetIndex]) {
    return current;
  }

  const nextTurns = [...current];
  nextTurns[targetIndex] = nextTurn;
  return nextTurns;
}

export function reduceTurnEventForTurn(
  current: ChatTurnView,
  event: DesktopTurnEvent
): ChatTurnView {
  const turnId = current.turnId ?? event.turnId;

  switch (event.type) {
    case "turn-started":
      return {
        ...current,
        turnId,
        status: "running",
      };
    case "llm-text-delta": {
      const text = getString(event.data.text);
      if (!text) {
        return current;
      }

      return {
        ...current,
        turnId,
        assistantText: `${current.assistantText}${text}`,
        status: keepTerminalTurnStatus(current.status),
      };
    }
    case "llm-reasoning-delta": {
      const text = getString(event.data.text);
      if (!text) {
        return current;
      }

      return {
        ...current,
        turnId,
        reasoningText: `${current.reasoningText}${text}`,
        status: keepTerminalTurnStatus(current.status),
      };
    }
    case "plan-updated": {
      const latestPlan = parsePlanSnapshot(event.data);
      if (!latestPlan) {
        return current;
      }

      return {
        ...current,
        latestPlan,
        turnId,
      };
    }
    case "tool-call-prepared": {
      const callId = getString(event.data.callId);
      const name = getString(event.data.name);
      const argumentsJson = getString(event.data.argumentsJson);
      if (!callId || !name || argumentsJson === null) {
        return current;
      }

      return {
        ...current,
        turnId,
        toolCalls: upsertToolCall(current.toolCalls, {
          argumentsJson,
          callId,
          name,
          status: "running",
        }),
      };
    }
    case "tool-call-completed": {
      const callId = getString(event.data.callId);
      const result = getRecord(event.data.result);
      if (!callId || !result) {
        return current;
      }

      return {
        ...current,
        turnId,
        toolCalls: upsertToolCall(
          current.toolCalls,
          mapCompletedToolCall(current.toolCalls, callId, result)
        ),
      };
    }
    case "tool-call-permission-requested": {
      const requestId = getString(event.data.requestId);
      const toolCallId = getString(event.data.toolCallId);
      if (!requestId || !toolCallId) {
        return current;
      }

      return {
        ...current,
        pendingPermissions: upsertPendingPermission(current.pendingPermissions, {
          requestId,
          toolCallId,
        }),
        turnId,
      };
    }
    case "tool-call-permission-resolved": {
      const requestId = getString(event.data.requestId);
      const decision = getString(event.data.decision);
      if (!requestId || !isPermissionDecision(decision)) {
        return current;
      }

      const resolved = current.pendingPermissions.find(
        (permission) => permission.requestId === requestId
      );
      if (!resolved) {
        return current;
      }

      return {
        ...current,
        lastResolvedPermission: {
          ...resolved,
          decision,
        },
        pendingPermissions: current.pendingPermissions.filter(
          (permission) => permission.requestId !== requestId
        ),
        turnId,
      };
    }
    case "turn-finished": {
      const reason = getString(event.data.reason);

      return {
        ...current,
        turnId,
        error:
          current.error ??
          (reason && reason !== "completed" && reason !== "cancelled"
            ? `Turn ended with ${reason}.`
            : null),
        lastResolvedPermission: null,
        pendingPermissions: [],
        status: mapTurnStatus(reason),
      };
    }
    case "turn-failed": {
      const message = getString(event.data.message);

      return {
        ...current,
        turnId,
        error: message ?? "Turn failed.",
        lastResolvedPermission: null,
        pendingPermissions: [],
        status: "failed",
      };
    }
    default:
      return {
        ...current,
        turnId,
      };
  }
}

function findTurnIndex(turns: ChatTurnView[], turnId: string) {
  const exactMatchIndex = turns.findIndex((turn) => turn.turnId === turnId);
  if (exactMatchIndex !== -1) {
    return exactMatchIndex;
  }

  for (let index = turns.length - 1; index >= 0; index -= 1) {
    if (turns[index].turnId === null && turns[index].status === "running") {
      return index;
    }
  }

  return -1;
}

function upsertToolCall(toolCalls: ToolCallView[], next: ToolCallView): ToolCallView[] {
  const existingIndex = toolCalls.findIndex((toolCall) => toolCall.callId === next.callId);
  if (existingIndex === -1) {
    return [...toolCalls, next];
  }

  const updated = [...toolCalls];
  updated[existingIndex] = {
    ...updated[existingIndex],
    ...next,
  };
  return updated;
}

function upsertPendingPermission(
  pendingPermissions: PendingPermissionView[],
  next: PendingPermissionView
): PendingPermissionView[] {
  const existingIndex = pendingPermissions.findIndex(
    (permission) => permission.requestId === next.requestId
  );
  if (existingIndex === -1) {
    return [...pendingPermissions, next];
  }

  const updated = [...pendingPermissions];
  updated[existingIndex] = next;
  return updated;
}

function clearResolvedPermission(
  turns: ChatTurnView[],
  turnId: string,
  requestId: string
): ChatTurnView[] {
  return turns.map((turn) =>
    turn.turnId === turnId &&
    turn.lastResolvedPermission?.requestId === requestId
      ? {
          ...turn,
          lastResolvedPermission: null,
        }
      : turn
  );
}

function selectVisiblePermission(turns: ChatTurnView[]) {
  const resolved = findLatestResolvedPermission(turns);
  if (resolved) {
    return resolved;
  }

  for (const turn of turns) {
    const pending = turn.pendingPermissions[0];
    if (!pending) {
      continue;
    }

    const toolCall = turn.toolCalls.find(
      (candidate) => candidate.callId === pending.toolCallId
    );

    return {
      argumentsSummary: toolCall
        ? formatArgumentsSummary(toolCall.argumentsJson)
        : undefined,
      requestId: pending.requestId,
      state: "requested" as const,
      toolCallId: pending.toolCallId,
      toolName: toolCall?.name ?? "tool",
      turnId: turn.turnId ?? turn.clientKey,
    };
  }

  return null;
}

function selectActiveFloatingPlan(turns: ChatTurnView[]) {
  for (let index = turns.length - 1; index >= 0; index -= 1) {
    const turn = turns[index];
    if (turn.status === "running" && turn.latestPlan) {
      return turn.latestPlan;
    }
  }

  return null;
}

function getRecentReadTasks(turn: ChatTurnView) {
  return turn.toolCalls
    .filter((toolCall) => isReadToolName(toolCall.name))
    .slice(-3)
    .map((toolCall) => ({
      callId: toolCall.callId,
      errorSummary: toolCall.errorSummary,
      inputSummary: summarizeReadToolInput(toolCall),
      name: toolCall.name,
      outputSummary: toolCall.outputSummary,
      status: toolCall.status,
    }));
}

function summarizeReadToolInput(toolCall: ToolCallView) {
  try {
    const parsed = JSON.parse(toolCall.argumentsJson) as Record<string, unknown>;
    const path = getString(parsed.path);
    const pattern = getString(parsed.pattern);

    if (path && pattern) {
      return `${path} · ${pattern}`;
    }

    if (path) {
      return path;
    }

    if (pattern) {
      return pattern;
    }
  } catch {
    return formatArgumentsSummary(toolCall.argumentsJson);
  }

  return formatArgumentsSummary(toolCall.argumentsJson);
}

function isReadToolName(name: string) {
  return name === "read" || name === "glob" || name === "grep";
}

function findLatestResolvedPermission(turns: ChatTurnView[]) {
  for (let index = turns.length - 1; index >= 0; index -= 1) {
    const turn = turns[index];
    const resolved = turn.lastResolvedPermission;
    if (!resolved) {
      continue;
    }

    const toolCall = turn.toolCalls.find(
      (candidate) => candidate.callId === resolved.toolCallId
    );

    return {
      argumentsSummary: toolCall
        ? formatArgumentsSummary(toolCall.argumentsJson)
        : undefined,
      requestId: resolved.requestId,
      state: resolved.decision === "allow" ? ("accepted" as const) : ("rejected" as const),
      toolCallId: resolved.toolCallId,
      toolName: toolCall?.name ?? "tool",
      turnId: turn.turnId ?? turn.clientKey,
    };
  }

  return null;
}

function mapCompletedToolCall(
  toolCalls: ToolCallView[],
  callId: string,
  result: Record<string, unknown>
): ToolCallView {
  const existing = toolCalls.find((toolCall) => toolCall.callId === callId);
  const status = getString(result.status);

  return {
    argumentsJson: existing?.argumentsJson ?? "{}",
    callId,
    errorSummary:
      status === "failed"
        ? getString(result.message) ?? summarizeValue(result)
        : status === "denied"
          ? "Tool call denied."
          : status === "timed_out"
            ? "Tool call timed out."
            : status === "cancelled"
              ? "Tool call cancelled."
              : undefined,
    name: existing?.name ?? "tool",
    outputSummary:
      status === "success" ? summarizeValue(result.output) : undefined,
    status: mapToolCallStatus(status),
  };
}

function mapToolCallStatus(status: string | null): ToolCallStatus {
  switch (status) {
    case "success":
      return "success";
    case "failed":
      return "failed";
    case "timed_out":
      return "timed_out";
    case "denied":
      return "denied";
    case "cancelled":
      return "cancelled";
    default:
      return "failed";
  }
}

function isPermissionDecision(value: string | null): value is PermissionDecision {
  return value === "allow" || value === "deny";
}

function mapTurnStatus(reason: string | null): TurnStatus {
  switch (reason) {
    case "completed":
      return "completed";
    case "cancelled":
      return "cancelled";
    case "failed":
      return "failed";
    default:
      return "failed";
  }
}

function keepTerminalTurnStatus(status: TurnStatus): TurnStatus {
  return status === "running" ? "running" : status;
}

function getString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function getRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }

  return value as Record<string, unknown>;
}

function getArray(value: unknown): unknown[] | null {
  return Array.isArray(value) ? value : null;
}

function parsePlanSnapshot(value: Record<string, unknown>): PlanSnapshot | null {
  const title = getString(value.title)?.trim();
  const sourceCallId = getString(value.sourceCallId)?.trim();
  const tasks = getArray(value.tasks)?.map(parsePlanTask);

  if (!title || !sourceCallId || !tasks || tasks.some((task) => task === null)) {
    return null;
  }

  return {
    description: getString(value.description),
    isStreaming: typeof value.isStreaming === "boolean" ? value.isStreaming : false,
    sourceCallId,
    tasks: tasks as PlanSnapshot["tasks"],
    title,
  };
}

function parsePlanTask(value: unknown): PlanSnapshot["tasks"][number] | null {
  const record = getRecord(value);
  const id = getString(record?.id)?.trim();
  const title = getString(record?.title)?.trim();
  const status = getString(record?.status);

  if (!id || !title || !isPlanTaskStatus(status)) {
    return null;
  }

  return {
    id,
    status,
    title,
  };
}

function isPlanTaskStatus(value: string | null): value is PlanSnapshot["tasks"][number]["status"] {
  return value === "pending" || value === "in_progress" || value === "completed";
}

function summarizeValue(value: unknown): string | undefined {
  if (value === undefined) {
    return undefined;
  }

  if (typeof value === "string") {
    return value;
  }

  return JSON.stringify(value);
}

function formatArgumentsSummary(argumentsJson: string): string {
  try {
    return JSON.stringify(JSON.parse(argumentsJson));
  } catch {
    return argumentsJson;
  }
}

function isNearBottom(element: HTMLDivElement) {
  return (
    element.scrollHeight - element.scrollTop - element.clientHeight <=
    AUTO_SCROLL_THRESHOLD_PX
  );
}
