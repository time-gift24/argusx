"use client";

import { useEffect, useMemo, useState } from "react";
import { useChatStore } from "@/lib/stores/chat-store";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface ChatFullInfoDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sessionId: string | null;
}

export function ChatFullInfoDialog({
  open,
  onOpenChange,
  sessionId,
}: ChatFullInfoDialogProps) {
  const loadSessionMessages = useChatStore((state) => state.loadSessionMessages);
  const loadFullSessionMessages = useChatStore((state) => state.loadFullSessionMessages);
  const messages = useChatStore((state) =>
    sessionId ? (state.messages[sessionId] ?? []) : []
  );
  const [loading, setLoading] = useState(false);

  const sortedMessages = useMemo(
    () => [...messages].sort((a, b) => a.createdAt - b.createdAt),
    [messages]
  );

  useEffect(() => {
    if (!open || !sessionId) {
      return;
    }

    let cancelled = false;
    setLoading(true);
    void loadSessionMessages(sessionId, { range: "all", limit: 300 })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [open, sessionId, loadSessionMessages]);

  const handleLoadOlder = async () => {
    if (!sessionId) {
      return;
    }
    setLoading(true);
    try {
      await loadFullSessionMessages(sessionId, 300);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="max-h-[80vh] max-w-4xl">
        <DialogHeader>
          <DialogTitle>Full Session Info</DialogTitle>
          <DialogDescription>
            This view loads complete history on demand and is not auto-expanded at startup.
          </DialogDescription>
        </DialogHeader>

        <div className="max-h-[55vh] overflow-y-auto rounded-md border bg-muted/20 p-3">
          {sortedMessages.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              {loading ? "Loading full history..." : "No messages available."}
            </p>
          ) : (
            <div className="space-y-2">
              {sortedMessages.map((message) => (
                <div
                  className="rounded-md border border-border/60 bg-background/80 px-3 py-2 text-sm"
                  key={message.id}
                >
                  <div className="mb-1 flex items-center justify-between text-xs text-muted-foreground">
                    <span>{message.role}</span>
                    <span>{new Date(message.createdAt).toLocaleString()}</span>
                  </div>
                  <p className="whitespace-pre-wrap break-words">{message.content}</p>
                </div>
              ))}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button disabled={!sessionId || loading} onClick={handleLoadOlder} type="button" variant="outline">
            {loading ? "Loading..." : "Load Older"}
          </Button>
          <Button onClick={() => onOpenChange(false)} type="button">
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
