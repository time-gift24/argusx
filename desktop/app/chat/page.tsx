"use client";

import {
  startTransition,
  useEffect,
  useEffectEvent,
  useRef,
  useState,
} from "react";
import { Streamdown } from "streamdown";

import {
  PromptComposer,
  Reasoning,
  ToolCallItem,
  type PromptComposerSubmitPayload,
} from "@/components/ai";
import {
  sharedStreamdownClassName,
  sharedStreamdownComponents,
  sharedStreamdownControls,
  sharedStreamdownIcons,
  sharedStreamdownPlugins,
  sharedStreamdownShikiTheme,
  sharedStreamdownTranslations,
} from "@/components/ai/streamdown";
import { cn } from "@/lib/utils";
import { useTurn, type DesktopTurnEvent } from "@/lib/chat";

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

type TurnStatus = "idle" | "running" | "completed" | "cancelled" | "failed";

type ToolCallView = {
  callId: string;
  name: string;
  argumentsJson: string;
  outputSummary?: string;
  errorSummary?: string;
  status: ToolCallStatus;
};

type ChatViewState = {
  activeTurnId: string | null;
  assistantText: string;
  error: string | null;
  prompt: string;
  reasoningText: string;
  status: TurnStatus;
  toolCalls: ToolCallView[];
};

const EMPTY_STATE: ChatViewState = {
  activeTurnId: null,
  assistantText: "",
  error: null,
  prompt: "",
  reasoningText: "",
  status: "idle",
  toolCalls: [],
};

export default function ChatPage() {
  const { cancelTurn, startTurn, subscribe } = useTurn();
  const [chatState, setChatState] = useState<ChatViewState>(EMPTY_STATE);
  const activeTurnIdRef = useRef<string | null>(null);
  const statusRef = useRef<TurnStatus>("idle");
  const ignoredTurnIdsRef = useRef(new Set<string>());

  useEffect(() => {
    activeTurnIdRef.current = chatState.activeTurnId;
    statusRef.current = chatState.status;
  }, [chatState.activeTurnId, chatState.status]);

  const handleTurnEvent = useEffectEvent((event: DesktopTurnEvent) => {
    if (ignoredTurnIdsRef.current.has(event.turnId)) {
      return;
    }

    startTransition(() => {
      setChatState((current) => reduceTurnEvent(current, event));
    });
  });

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
  }, [handleTurnEvent, subscribe]);

  const handleSubmit = async (payload: PromptComposerSubmitPayload) => {
    if (payload.category !== "agent") {
      throw new Error("Workflow turns are not implemented yet.");
    }

    const previousTurnId = activeTurnIdRef.current;
    if (previousTurnId) {
      ignoredTurnIdsRef.current.add(previousTurnId);
    }

    if (previousTurnId && statusRef.current === "running") {
      await cancelTurn(previousTurnId);
    }

    startTransition(() => {
      setChatState({
        ...EMPTY_STATE,
        prompt: payload.draft,
        status: "running",
      });
    });

    try {
      const result = await startTurn({
        prompt: payload.draft,
        targetId: payload.selectionId,
        targetKind: "agent",
      });

      ignoredTurnIdsRef.current.delete(result.turnId);

      startTransition(() => {
        setChatState((current) => ({
          ...current,
          activeTurnId: current.activeTurnId ?? result.turnId,
          status: "running",
        }));
      });
    } catch (error) {
      startTransition(() => {
        setChatState((current) => ({
          ...current,
          error: error instanceof Error ? error.message : "Unable to start turn.",
          status: "failed",
        }));
      });
      throw error;
    }
  };

  const handleCancel = async () => {
    if (!chatState.activeTurnId || chatState.status !== "running") {
      return;
    }

    await cancelTurn(chatState.activeTurnId);
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col p-4 lg:p-6">
      <div className="mx-auto flex min-h-0 w-full max-w-5xl flex-1 flex-col gap-4">
        <div className="min-h-0 flex-1 overflow-y-auto">
          <div className="flex min-h-full flex-col justify-end">
            {hasConversationState(chatState) ? (
              <div className="rounded-2xl border border-border/70 bg-card/80 p-4 shadow-sm">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <p className="text-[11px] font-medium uppercase tracking-[0.08em] text-muted-foreground">
                      Latest Turn
                    </p>
                    <p className="mt-1 text-sm text-foreground/80">
                      {chatState.prompt || "Waiting for input"}
                    </p>
                  </div>
                  {chatState.status === "running" && chatState.activeTurnId ? (
                    <button
                      className="rounded-full border border-border/60 px-3 py-1 text-xs text-foreground transition-colors hover:bg-muted/60"
                      onClick={() => void handleCancel()}
                      type="button"
                    >
                      Cancel
                    </button>
                  ) : null}
                </div>

                <div className="mt-4 flex flex-col gap-4">
                  {chatState.assistantText ? (
                    <Streamdown
                      className={cn(sharedStreamdownClassName, "text-sm")}
                      components={sharedStreamdownComponents}
                      controls={sharedStreamdownControls}
                      icons={sharedStreamdownIcons}
                      isAnimating={chatState.status === "running"}
                      plugins={sharedStreamdownPlugins}
                      shikiTheme={sharedStreamdownShikiTheme}
                      translations={sharedStreamdownTranslations}
                    >
                      {chatState.assistantText}
                    </Streamdown>
                  ) : chatState.status === "running" ? (
                    <p className="text-sm text-muted-foreground">
                      Waiting for model output...
                    </p>
                  ) : null}

                  {chatState.reasoningText ? (
                    <Reasoning
                      isRunning={chatState.status === "running"}
                      runKey={chatState.activeTurnId ?? chatState.prompt}
                    >
                      {chatState.reasoningText}
                    </Reasoning>
                  ) : null}

                  {chatState.toolCalls.map((toolCall) => (
                    <ToolCallItem
                      inputSummary={formatArgumentsSummary(toolCall.argumentsJson)}
                      isRunning={toolCall.status === "running"}
                      key={toolCall.callId}
                      name={toolCall.name}
                      outputSummary={toolCall.outputSummary}
                      runKey={chatState.activeTurnId ?? chatState.prompt}
                      errorSummary={toolCall.errorSummary}
                    />
                  ))}

                  {chatState.error ? (
                    <p
                      className="text-sm text-destructive"
                      role="alert"
                    >
                      {chatState.error}
                    </p>
                  ) : null}
                </div>
              </div>
            ) : (
              <div className="flex min-h-full items-end">
                <p className="text-sm text-muted-foreground">
                  Start a turn to stream assistant output, reasoning, and tool activity.
                </p>
              </div>
            )}
          </div>
        </div>

        <PromptComposer
          agents={[...AGENTS]}
          onSubmit={handleSubmit}
          workflows={[]}
        />
      </div>
    </div>
  );
}

function hasConversationState(state: ChatViewState) {
  return (
    state.prompt.length > 0 ||
    state.assistantText.length > 0 ||
    state.reasoningText.length > 0 ||
    state.toolCalls.length > 0 ||
    state.error !== null
  );
}

function reduceTurnEvent(
  current: ChatViewState,
  event: DesktopTurnEvent
): ChatViewState {
  if (current.activeTurnId && current.activeTurnId !== event.turnId) {
    return current;
  }

  if (!current.activeTurnId && current.status === "idle" && current.prompt.length === 0) {
    return current;
  }

  const activeTurnId = current.activeTurnId ?? event.turnId;

  switch (event.type) {
    case "turn-started":
      return {
        ...current,
        activeTurnId,
        status: "running",
      };
    case "llm-text-delta": {
      const text = getString(event.data.text);
      if (!text) {
        return current;
      }

      return {
        ...current,
        activeTurnId,
        assistantText: `${current.assistantText}${text}`,
        status: "running",
      };
    }
    case "llm-reasoning-delta": {
      const text = getString(event.data.text);
      if (!text) {
        return current;
      }

      return {
        ...current,
        activeTurnId,
        reasoningText: `${current.reasoningText}${text}`,
        status: "running",
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
        activeTurnId,
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
        activeTurnId,
        toolCalls: upsertToolCall(
          current.toolCalls,
          mapCompletedToolCall(current.toolCalls, callId, result)
        ),
      };
    }
    case "turn-finished": {
      const reason = getString(event.data.reason);

      return {
        ...current,
        activeTurnId,
        error:
          current.error ??
          (reason && reason !== "completed" && reason !== "cancelled"
            ? `Turn ended with ${reason}.`
            : null),
        status: mapTurnStatus(reason),
      };
    }
    case "turn-failed": {
      const message = getString(event.data.message);

      return {
        ...current,
        activeTurnId,
        error: message ?? "Turn failed.",
        status: "failed",
      };
    }
    default:
      return {
        ...current,
        activeTurnId,
      };
  }
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

function getString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function getRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }

  return value as Record<string, unknown>;
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
