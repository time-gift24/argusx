"use client";

import { startTransition, useEffect, useEffectEvent, useState } from "react";

import { PromptComposer } from "@/components/ai";
import { Button } from "@/components/ui/button";
import {
  CHAT_AGENTS,
  CHAT_WORKFLOWS,
  cancelConversation,
  continueConversation,
  listConversationThreads,
  restartConversation,
  startConversation,
  subscribeToTurnEvents,
  switchConversationThread,
  type ConversationThreadSummary,
  type DesktopTurnEvent,
} from "@/lib/chat";
import { cn } from "@/lib/utils";

type ToolStatus =
  | "running"
  | "success"
  | "error"
  | "denied"
  | "cancelled"
  | "timed_out";

type TurnStatus = "running" | "completed" | "failed" | "cancelled";

type ToolView = {
  argumentsText?: string;
  callId: string;
  errorText?: string;
  name: string;
  outputText?: string;
  status: ToolStatus;
};

type TurnView = {
  assistantText: string;
  reasoningText: string;
  status: TurnStatus;
  tools: ToolView[];
  turnId: string;
  userPrompt: string;
};

function createEmptyTurn(turnId: string): TurnView {
  return {
    assistantText: "",
    reasoningText: "",
    status: "running",
    tools: [],
    turnId,
    userPrompt: "",
  };
}

function updateTurn(
  turns: TurnView[],
  turnId: string,
  transform: (turn: TurnView) => TurnView
): TurnView[] {
  let updated = false;
  const nextTurns = turns.map((turn) => {
    if (turn.turnId !== turnId) {
      return turn;
    }

    updated = true;
    return transform(turn);
  });

  if (updated) {
    return nextTurns;
  }

  return [...nextTurns, transform(createEmptyTurn(turnId))];
}

function updateTool(
  tools: ToolView[],
  callId: string,
  transform: (tool: ToolView) => ToolView
): ToolView[] {
  let updated = false;
  const nextTools = tools.map((tool) => {
    if (tool.callId !== callId) {
      return tool;
    }

    updated = true;
    return transform(tool);
  });

  if (updated) {
    return nextTools;
  }

  return [
    ...nextTools,
    transform({
      callId,
      name: callId,
      status: "running",
    }),
  ];
}

function stringifyValue(value: unknown): string | undefined {
  if (value == null) {
    return undefined;
  }

  if (typeof value === "string") {
    return value;
  }

  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function turnStatusFromReason(reason: unknown): TurnStatus {
  switch (reason) {
    case "cancelled":
      return "cancelled";
    case "failed":
      return "failed";
    default:
      return "completed";
  }
}

function toolStatusFromEvent(status: unknown): ToolStatus {
  switch (status) {
    case "success":
    case "error":
    case "denied":
    case "cancelled":
    case "timed_out":
      return status;
    default:
      return "running";
  }
}

function applyTurnEvent(turns: TurnView[], event: DesktopTurnEvent): TurnView[] {
  switch (event.type) {
    case "turn-started":
      return updateTurn(turns, event.turnId, (turn) => ({
        ...turn,
        status: "running",
      }));
    case "llm-text-delta":
      return updateTurn(turns, event.turnId, (turn) => ({
        ...turn,
        assistantText: `${turn.assistantText}${String(event.data.text ?? "")}`,
      }));
    case "llm-reasoning-delta":
      return updateTurn(turns, event.turnId, (turn) => ({
        ...turn,
        reasoningText: `${turn.reasoningText}${String(event.data.text ?? "")}`,
      }));
    case "tool-call-prepared":
      return updateTurn(turns, event.turnId, (turn) => ({
        ...turn,
        tools: updateTool(
          turn.tools,
          String(event.data.callId ?? event.turnId),
          (tool) => ({
            ...tool,
            argumentsText: stringifyValue(event.data.arguments),
            name: String(event.data.name ?? tool.name),
            status: "running",
          })
        ),
      }));
    case "tool-call-completed":
      return updateTurn(turns, event.turnId, (turn) => ({
        ...turn,
        tools: updateTool(
          turn.tools,
          String(event.data.callId ?? event.turnId),
          (tool) => ({
            ...tool,
            errorText:
              stringifyValue((event.data.error as { message?: unknown } | undefined)?.message) ??
              tool.errorText,
            outputText: stringifyValue(event.data.output) ?? tool.outputText,
            status: toolStatusFromEvent(event.data.status),
          })
        ),
      }));
    case "turn-finished":
      return updateTurn(turns, event.turnId, (turn) => ({
        ...turn,
        status: turnStatusFromReason(event.data.reason),
      }));
    default:
      return turns;
  }
}

function TranscriptTurn({ turn }: { turn: TurnView }) {
  return (
    <article
      className="rounded-3xl border border-border/60 bg-background/80 p-4 shadow-sm backdrop-blur"
      data-turn-id={turn.turnId}
    >
      <div className="flex items-center justify-between gap-3">
        <span className="text-[11px] font-medium uppercase tracking-[0.16em] text-muted-foreground">
          Turn
        </span>
        <span
          className={cn(
            "rounded-full border px-2 py-0.5 text-[11px] font-medium capitalize",
            turn.status === "running" && "border-primary/30 text-primary",
            turn.status === "completed" && "border-emerald-500/20 text-emerald-700",
            turn.status === "failed" && "border-destructive/20 text-destructive",
            turn.status === "cancelled" && "border-border text-muted-foreground"
          )}
        >
          {turn.status}
        </span>
      </div>
      <div className="mt-4 space-y-4">
        <section className="space-y-1">
          <p className="text-[11px] font-medium uppercase tracking-[0.16em] text-muted-foreground">
            Prompt
          </p>
          <p className="whitespace-pre-wrap text-sm leading-6 text-foreground">
            {turn.userPrompt || "Waiting for prompt metadata..."}
          </p>
        </section>

        {turn.reasoningText ? (
          <section className="space-y-1">
            <p className="text-[11px] font-medium uppercase tracking-[0.16em] text-muted-foreground">
              Reasoning
            </p>
            <pre className="overflow-x-auto whitespace-pre-wrap rounded-2xl border border-border/50 bg-muted/40 p-3 text-xs leading-5 text-muted-foreground">
              {turn.reasoningText}
            </pre>
          </section>
        ) : null}

        {turn.tools.length > 0 ? (
          <section className="space-y-2">
            <p className="text-[11px] font-medium uppercase tracking-[0.16em] text-muted-foreground">
              Tools
            </p>
            <div className="space-y-2">
              {turn.tools.map((tool) => (
                <div
                  className="rounded-2xl border border-border/50 bg-muted/30 p-3"
                  key={tool.callId}
                >
                  <div className="flex items-center justify-between gap-3">
                    <span className="font-mono text-xs text-foreground">{tool.name}</span>
                    <span className="text-[11px] capitalize text-muted-foreground">
                      {tool.status}
                    </span>
                  </div>
                  {tool.argumentsText ? (
                    <pre className="mt-2 overflow-x-auto whitespace-pre-wrap text-xs leading-5 text-muted-foreground">
                      {tool.argumentsText}
                    </pre>
                  ) : null}
                  {tool.outputText ? (
                    <pre className="mt-2 overflow-x-auto whitespace-pre-wrap text-xs leading-5 text-foreground">
                      {tool.outputText}
                    </pre>
                  ) : null}
                  {tool.errorText ? (
                    <p className="mt-2 text-xs leading-5 text-destructive">{tool.errorText}</p>
                  ) : null}
                </div>
              ))}
            </div>
          </section>
        ) : null}

        {turn.assistantText ? (
          <section className="space-y-1">
            <p className="text-[11px] font-medium uppercase tracking-[0.16em] text-muted-foreground">
              Assistant
            </p>
            <p className="whitespace-pre-wrap text-sm leading-6 text-foreground">
              {turn.assistantText}
            </p>
          </section>
        ) : null}
      </div>
    </article>
  );
}

export default function ChatPage() {
  const [conversationId, setConversationId] = useState<string | null>(null);
  const [activeTurnId, setActiveTurnId] = useState<string | null>(null);
  const [threads, setThreads] = useState<ConversationThreadSummary[]>([]);
  const [turns, setTurns] = useState<TurnView[]>([]);
  const [error, setError] = useState<string | null>(null);
  const selectedThread =
    threads.find((thread) => thread.conversationId === conversationId) ?? null;

  const refreshThreads = useEffectEvent(async () => {
    try {
      const listed = await listConversationThreads();
      startTransition(() => {
        setThreads(listed);
        setConversationId(
          (current) =>
            current ??
            listed.find((thread) => thread.isActive)?.conversationId ??
            listed[0]?.conversationId ??
            null
        );
      });
    } catch {
      setError((current) => current ?? "Unable to load conversation threads.");
    }
  });

  const handleTurnEvent = useEffectEvent((event: DesktopTurnEvent) => {
    if (conversationId && event.conversationId !== conversationId) {
      return;
    }

    startTransition(() => {
      setConversationId((current) => current ?? event.conversationId);
      setTurns((current) => applyTurnEvent(current, event));

      if (event.type === "turn-finished") {
        setActiveTurnId((current) => (current === event.turnId ? null : current));
      } else {
        setActiveTurnId((current) => current ?? event.turnId);
      }
    });

    if (event.type === "turn-finished") {
      void refreshThreads();
    }
  });

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void subscribeToTurnEvents(handleTurnEvent).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      unlisten?.();
    };
  }, [handleTurnEvent]);

  useEffect(() => {
    void refreshThreads();
  }, [refreshThreads]);

  const handleCancel = async () => {
    if (!conversationId) {
      return;
    }

    try {
      await cancelConversation({ conversationId });
      setError(null);
    } catch {
      setError("Unable to cancel the active turn.");
    }
  };

  const handleThreadSwitch = async (nextConversationId: string) => {
    try {
      setError(null);
      await switchConversationThread({ conversationId: nextConversationId });
      startTransition(() => {
        setConversationId(nextConversationId);
        setActiveTurnId(null);
        setTurns([]);
      });
      await refreshThreads();
    } catch {
      setError("Unable to switch conversation threads.");
    }
  };

  const handleRestart = async () => {
    if (!conversationId) {
      return;
    }

    try {
      setError(null);
      const started = await restartConversation({ conversationId });
      startTransition(() => {
        setConversationId(started.conversationId);
        setActiveTurnId(started.turnId);
        setTurns((current) =>
          updateTurn(current, started.turnId, (turn) => ({
            ...turn,
            status: "running",
            userPrompt: turn.userPrompt || "Restarting interrupted turn...",
          }))
        );
      });
      await refreshThreads();
    } catch {
      setError("Unable to restart the interrupted turn.");
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-[radial-gradient(circle_at_top,_rgba(22,163,74,0.08),_transparent_38%),linear-gradient(180deg,_rgba(255,255,255,0.96),_rgba(248,250,252,0.92))] p-4 lg:p-6">
      <div className="mx-auto flex min-h-0 w-full max-w-5xl flex-1 flex-col gap-4">
        {threads.length > 0 ? (
          <section className="grid gap-2 rounded-[28px] border border-border/60 bg-background/70 p-3 shadow-sm backdrop-blur sm:grid-cols-2 lg:grid-cols-3">
            {threads.map((thread) => (
              <button
                className={cn(
                  "rounded-2xl border px-3 py-3 text-left transition-colors",
                  thread.conversationId === conversationId
                    ? "border-primary/40 bg-primary/5"
                    : "border-border/60 bg-muted/20 hover:bg-muted/40"
                )}
                key={thread.conversationId}
                onClick={() => void handleThreadSwitch(thread.conversationId)}
                type="button"
              >
                <div className="flex items-center justify-between gap-3">
                  <span className="truncate text-sm font-medium text-foreground">
                    {thread.title}
                  </span>
                  <span className="text-[11px] capitalize text-muted-foreground">
                    {thread.status.replaceAll("_", " ")}
                  </span>
                </div>
                <p className="mt-1 truncate text-xs text-muted-foreground">
                  {thread.targetKind} / {thread.targetId}
                </p>
              </button>
            ))}
          </section>
        ) : null}

        <section className="min-h-0 flex-1 overflow-y-auto rounded-[28px] border border-border/60 bg-background/70 p-4 shadow-sm backdrop-blur">
          {turns.length > 0 ? (
            <div className="space-y-4">
              {turns.map((turn) => (
                <TranscriptTurn key={turn.turnId} turn={turn} />
              ))}
            </div>
          ) : (
            <div className="flex h-full min-h-56 items-center justify-center rounded-[24px] border border-dashed border-border/70 bg-muted/20 px-6 text-center text-sm text-muted-foreground">
              Start a conversation with an agent or workflow. Streaming output will appear here.
            </div>
          )}
        </section>

        {error ? (
          <p className="text-sm text-destructive" role="alert">
            {error}
          </p>
        ) : null}

        {selectedThread?.status === "restartable" && !activeTurnId ? (
          <div className="flex justify-end">
            <Button onClick={() => void handleRestart()} size="sm" variant="outline">
              Restart Turn
            </Button>
          </div>
        ) : null}

        {activeTurnId && conversationId ? (
          <div className="flex justify-end">
            <Button onClick={() => void handleCancel()} size="sm" variant="outline">
              Cancel Turn
            </Button>
          </div>
        ) : null}

        <PromptComposer
          agents={[...CHAT_AGENTS]}
          onSubmit={async ({ category, draft, selectionId }) => {
            if (activeTurnId) {
              throw new Error("conversation already has an active turn");
            }

            setError(null);

            const started = conversationId
              ? await continueConversation({
                  conversationId,
                  prompt: draft,
                })
              : await startConversation({
                  prompt: draft,
                  targetId: selectionId,
                  targetKind: category,
                });

            startTransition(() => {
              setConversationId(started.conversationId);
              setActiveTurnId(started.turnId);
              setTurns((current) =>
                updateTurn(current, started.turnId, (turn) => ({
                  ...turn,
                  status: "running",
                  userPrompt: draft,
                }))
              );
            });
            await refreshThreads();
          }}
          workflows={[...CHAT_WORKFLOWS]}
        />
      </div>
    </div>
  );
}
