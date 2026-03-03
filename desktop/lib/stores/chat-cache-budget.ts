export interface CacheSessionMeta {
  id: string;
  updatedAt: number;
}

export interface CacheMessageLike {
  createdAt: number;
}

export interface CacheTurnLike {
  createdAt: number;
  updatedAt: number;
}

export interface TrimmedCacheResult<M extends CacheMessageLike, T extends CacheTurnLike> {
  messages: Record<string, M[]>;
  turns: Record<string, T[]>;
  estimatedBytes: number;
}

const textEncoder = new TextEncoder();

function estimateBytes<M extends CacheMessageLike, T extends CacheTurnLike>(
  messages: Record<string, M[]>,
  turns: Record<string, T[]>
): number {
  const payload = JSON.stringify({ messages, turns });
  return textEncoder.encode(payload).length;
}

type EvictionCandidate =
  | { kind: "message"; sessionId: string; createdAt: number }
  | { kind: "turn"; sessionId: string; createdAt: number };

function findOldestCandidate<M extends CacheMessageLike, T extends CacheTurnLike>(
  messages: Record<string, M[]>,
  turns: Record<string, T[]>
): EvictionCandidate | undefined {
  let oldest: EvictionCandidate | undefined;
  const isOlder = (createdAt: number): boolean =>
    oldest === undefined || createdAt < oldest.createdAt;

  for (const [sessionId, sessionMessages] of Object.entries(messages)) {
    const first = sessionMessages[0];
    if (first && isOlder(first.createdAt)) {
      oldest = {
        kind: "message",
        sessionId,
        createdAt: first.createdAt,
      };
    }
  }

  for (const [sessionId, sessionTurns] of Object.entries(turns)) {
    const first = sessionTurns[0];
    if (first && isOlder(first.createdAt)) {
      oldest = {
        kind: "turn",
        sessionId,
        createdAt: first.createdAt,
      };
    }
  }

  return oldest;
}

export function trimChatCacheToBudget<M extends CacheMessageLike, T extends CacheTurnLike>(
  sessions: CacheSessionMeta[],
  messages: Record<string, M[]>,
  turns: Record<string, T[]>,
  _activeSessionId: string | null,
  budgetBytes: number
): TrimmedCacheResult<M, T> {
  void sessions;
  void _activeSessionId;
  const nextMessages: Record<string, M[]> = Object.fromEntries(
    Object.entries(messages).map(([sessionId, values]) => [sessionId, [...values]])
  );
  const nextTurns: Record<string, T[]> = Object.fromEntries(
    Object.entries(turns).map(([sessionId, values]) => [sessionId, [...values]])
  );

  let estimatedBytes = estimateBytes(nextMessages, nextTurns);
  if (estimatedBytes <= budgetBytes) {
    return { messages: nextMessages, turns: nextTurns, estimatedBytes };
  }

  while (estimatedBytes > budgetBytes) {
    const oldest = findOldestCandidate(nextMessages, nextTurns);
    if (!oldest) {
      break;
    }

    if (oldest.kind === "message") {
      nextMessages[oldest.sessionId] = (nextMessages[oldest.sessionId] ?? []).slice(
        1
      );
    } else {
      nextTurns[oldest.sessionId] = (nextTurns[oldest.sessionId] ?? []).slice(1);
    }

    estimatedBytes = estimateBytes(nextMessages, nextTurns);
  }

  return { messages: nextMessages, turns: nextTurns, estimatedBytes };
}
