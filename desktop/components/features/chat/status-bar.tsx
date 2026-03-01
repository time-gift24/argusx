"use client";

import { useChatStore } from "@/lib/stores/chat-store";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { StopCircle } from "lucide-react";

export function StatusBar() {
  const { agentStatus, currentSessionId, stopAgent } = useChatStore();

  const handleStop = async () => {
    await stopAgent();
  };

  return (
    <div className="flex items-center justify-between border-t px-4 py-1 text-sm">
      <span className="text-muted-foreground">
        Status:{" "}
        <span
          className={cn(
            agentStatus === "idle" && "text-green-500",
            agentStatus === "running" && "text-blue-500",
            agentStatus === "error" && "text-red-500"
          )}
        >
          {agentStatus}
        </span>
        {currentSessionId && ` (session: ${currentSessionId.slice(0, 8)})`}
      </span>

      {agentStatus === "running" && (
        <Button variant="outline" size="sm" onClick={handleStop}>
          <StopCircle className="w-4 h-4 mr-1" />
          停止
        </Button>
      )}
    </div>
  );
}
