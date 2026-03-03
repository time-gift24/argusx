import type { AgentTurnVM, QueueItemVM, ToolCallVM } from "@/lib/stores/chat-store";

export type TurnProcessSectionKey = "reasoning" | "plan" | "tools" | "terminal";

export type TurnProcessStatus =
  | "thinking"
  | "tool-call"
  | "outputing"
  | "done"
  | "failed";

export interface TurnProcessSectionVM {
  key: TurnProcessSectionKey;
  title: string;
  preview: string;
  headerLabel: string;
  headerDetail?: string;
  compactItems?: TurnProcessCompactItemVM[];
  isStreaming: boolean;
  defaultOpen: boolean;
}

export interface TurnProcessCompactItemVM {
  id: string;
  label: string;
  status: "Waiting" | "Running" | "Completed" | "Failed";
}

export interface TurnProcessMetrics {
  toolCount: number;
  queue: {
    waiting: number;
    running: number;
    completed: number;
    failed: number;
  };
  terminalLines: number;
  durationMs?: number;
}

export interface TurnProcessVM {
  hasProcess: boolean;
  status: TurnProcessStatus;
  statusLabel: string;
  summary: string;
  metrics: TurnProcessMetrics;
  sections: TurnProcessSectionVM[];
  terminalOutput: string;
}

const TOOL_RUNNING_STATES = new Set<ToolCallVM["state"]>([
  "input-available",
  "input-streaming",
]);

const statusLabel: Record<TurnProcessStatus, string> = {
  thinking: "Thinking",
  "tool-call": "Running",
  outputing: "Writing",
  done: "Completed",
  failed: "Failed",
};

const queueStatusLabel: Record<QueueItemVM["status"], string> = {
  waiting: "waiting",
  running: "running",
  completed: "completed",
  failed: "failed",
};

const queueStatusDisplayLabel: Record<
  QueueItemVM["status"],
  TurnProcessCompactItemVM["status"]
> = {
  waiting: "Waiting",
  running: "Running",
  completed: "Completed",
  failed: "Failed",
};

const toSingleLine = (value: string): string =>
  value.replace(/\s+/g, " ").trim();

const toPreview = (value: string, max = 120): string => {
  const compact = toSingleLine(value);
  const chars = Array.from(compact);
  if (chars.length <= max) {
    return compact;
  }
  return `${chars.slice(0, max).join("")}...`;
};

const formatDuration = (durationMs: number): string => {
  if (durationMs < 1000) {
    return `${durationMs}ms`;
  }
  const seconds = durationMs / 1000;
  if (seconds < 10) {
    return `${seconds.toFixed(1)}s`;
  }
  if (seconds < 60) {
    return `${Math.round(seconds)}s`;
  }
  const minutes = Math.floor(seconds / 60);
  const remainder = Math.round(seconds % 60);
  return `${minutes}m ${remainder}s`;
};

const countLines = (value: string): number => {
  if (!value.trim()) {
    return 0;
  }
  return value.split("\n").length;
};

const buildTerminalOutput = (turn: AgentTurnVM): string => {
  const sections: string[] = [];
  if (turn.terminal.stdout.trim().length > 0) {
    sections.push(`stdout:\n${turn.terminal.stdout}`);
  }
  if (turn.terminal.stderr.trim().length > 0) {
    sections.push(`stderr:\n${turn.terminal.stderr}`);
  }
  if (sections.length === 0 && turn.terminal.output.trim().length > 0) {
    sections.push(turn.terminal.output);
  }
  if (turn.terminal.exitCode !== undefined) {
    sections.push(
      `exit_code: ${turn.terminal.exitCode} (${turn.terminal.durationMs ?? 0}ms)`
    );
  }
  return sections.join("\n\n");
};

const resolveQueueMetrics = (items: AgentTurnVM["queue"]["items"]) => {
  const metrics = {
    waiting: 0,
    running: 0,
    completed: 0,
    failed: 0,
  };

  for (const item of items) {
    metrics[item.status] += 1;
  }

  return metrics;
};

const toolStateToCompactStatus = (
  state: ToolCallVM["state"]
): TurnProcessCompactItemVM["status"] => {
  if (state === "input-available" || state === "input-streaming") {
    return "Running";
  }
  if (state === "output-error" || state === "output-denied") {
    return "Failed";
  }
  if (state === "output-available" || state === "approval-responded") {
    return "Completed";
  }
  return "Waiting";
};

const buildToolCompactItems = (turn: AgentTurnVM): TurnProcessCompactItemVM[] => {
  const itemsFromQueue = [...turn.queue.items]
    .sort((a, b) => b.updatedAt - a.updatedAt)
    .map((item) => ({
      id: item.callId,
      label: item.toolName,
      status: queueStatusDisplayLabel[item.status],
    }));

  if (itemsFromQueue.length > 0) {
    return itemsFromQueue;
  }

  return [...turn.tools]
    .sort((a, b) => b.updatedAt - a.updatedAt)
    .map((tool) => ({
      id: tool.callId,
      label: tool.toolName,
      status: toolStateToCompactStatus(tool.state),
    }));
};

const buildToolsHeaderDetail = (items: TurnProcessCompactItemVM[]): string | undefined => {
  if (items.length === 0) {
    return undefined;
  }

  const first = items[0]?.label;
  if (!first) {
    return undefined;
  }

  if (items.length === 1) {
    return first;
  }

  return `${first} +${items.length - 1}`;
};

const resolveStatus = (turn: AgentTurnVM): TurnProcessStatus => {
  if (turn.status === "failed" || turn.status === "cancelled") {
    return "failed";
  }

  const hasToolActivity =
    turn.terminal.isStreaming ||
    turn.queue.items.some(
      (item) => item.status === "running" || item.status === "waiting"
    ) ||
    turn.tools.some((tool) => TOOL_RUNNING_STATES.has(tool.state));

  if (hasToolActivity) {
    return "tool-call";
  }

  if (turn.reasoning.isStreaming) {
    return "thinking";
  }

  if (turn.status === "streaming") {
    return "outputing";
  }

  if (turn.status === "done") {
    return "done";
  }

  return "thinking";
};

export const buildTurnProcessVM = (turn: AgentTurnVM): TurnProcessVM => {
  const terminalOutput = buildTerminalOutput(turn);
  const terminalLines = countLines(terminalOutput);
  const queue = resolveQueueMetrics(turn.queue.items);
  const status = resolveStatus(turn);
  const sections: TurnProcessSectionVM[] = [];

  const hasReasoning =
    turn.reasoning.text.trim().length > 0 || turn.reasoning.isStreaming;
  if (hasReasoning) {
    sections.push({
      key: "reasoning",
      title: "Reasoning",
      preview:
        turn.reasoning.preview ||
        (turn.reasoning.isStreaming
          ? "Streaming reasoning..."
          : "Reasoning captured"),
      headerLabel: turn.reasoning.isStreaming ? "Thinking..." : "Reasoning",
      isStreaming: turn.reasoning.isStreaming,
      defaultOpen: false,
    });
  }

  if (turn.plan && turn.plan.tasks.length > 0) {
    const completed = turn.plan.tasks.filter(
      (task) => task.status === "completed"
    ).length;
    sections.push({
      key: "plan",
      title: "Plan",
      preview: `${completed}/${turn.plan.tasks.length} completed`,
      headerLabel: turn.plan.isStreaming ? "Planning..." : "Plan",
      isStreaming: turn.plan.isStreaming,
      defaultOpen: turn.plan.isStreaming,
    });
  }

  if (turn.tools.length > 0 || turn.queue.items.length > 0) {
    const compactItems = buildToolCompactItems(turn);
    const isStreaming =
      queue.running > 0 || queue.waiting > 0 || turn.tools.some((tool) => TOOL_RUNNING_STATES.has(tool.state));
    const latest = [...turn.queue.items]
      .sort((a, b) => b.updatedAt - a.updatedAt)
      .at(0);
    const queuePreview = latest
      ? `${latest.toolName} · ${queueStatusLabel[latest.status]}`
      : `${turn.tools.length} tools`;
    sections.push({
      key: "tools",
      title: "Tools",
      preview: queuePreview,
      headerLabel: isStreaming ? "Running tools..." : "Tools",
      headerDetail: buildToolsHeaderDetail(compactItems),
      compactItems,
      isStreaming,
      defaultOpen: false,
    });
  }

  if (terminalOutput.trim().length > 0 || turn.terminal.isStreaming) {
    const firstLine = terminalOutput
      .split("\n")
      .map((line) => line.trim())
      .find((line) => line.length > 0);
    sections.push({
      key: "terminal",
      title: "Terminal",
      preview: firstLine ? toPreview(firstLine, 88) : "Streaming terminal output...",
      headerLabel: turn.terminal.isStreaming ? "Running terminal..." : "Terminal",
      headerDetail: firstLine ? toPreview(firstLine, 64) : undefined,
      isStreaming: turn.terminal.isStreaming,
      defaultOpen: false,
    });
  }

  const metrics: TurnProcessMetrics = {
    toolCount: turn.tools.length,
    queue,
    terminalLines,
    durationMs: turn.terminal.durationMs,
  };

  const summarySegments: string[] = [statusLabel[status]];
  if (metrics.toolCount > 0) {
    summarySegments.push(`tools ${metrics.toolCount}`);
  }
  if (metrics.queue.running > 0) {
    summarySegments.push(`running ${metrics.queue.running}`);
  } else if (metrics.queue.waiting > 0) {
    summarySegments.push(`queued ${metrics.queue.waiting}`);
  } else if (metrics.queue.failed > 0) {
    summarySegments.push(`failed ${metrics.queue.failed}`);
  }
  if (metrics.terminalLines > 0) {
    summarySegments.push(`terminal ${metrics.terminalLines} lines`);
  }
  if (metrics.durationMs !== undefined) {
    summarySegments.push(formatDuration(metrics.durationMs));
  }

  return {
    hasProcess: sections.length > 0,
    status,
    statusLabel: statusLabel[status],
    summary: summarySegments.join(" · "),
    metrics,
    sections,
    terminalOutput,
  };
};
