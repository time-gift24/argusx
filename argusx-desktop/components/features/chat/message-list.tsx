"use client";

import { useChatStore } from "@/lib/stores/chat-store";
import { cn } from "@/lib/utils";

export function MessageList() {
  const { messages, reasoningText, toolProgress, agentStatus } = useChatStore();

  return (
    <div className="flex-1 overflow-auto p-4 space-y-4">
      {messages.map((msg) => (
        <div
          key={msg.id}
          className={cn(
            "max-w-[80%]",
            msg.role === "user" ? "ml-auto text-right" : "mr-auto text-left"
          )}
        >
          <span className="text-xs font-bold text-muted-foreground">
            {msg.role === "user" ? "User" : msg.role === "assistant" ? "Agent" : "System"}
          </span>
          <div
            className={cn(
              "rounded-lg p-3 mt-1",
              msg.role === "user" ? "bg-primary text-primary-foreground" : "bg-muted"
            )}
          >
            {msg.content || <span className="italic text-muted-foreground">...</span>}
          </div>
        </div>
      ))}

      {/* Reasoning 显示 */}
      {reasoningText && agentStatus === "running" && (
        <div className="mr-auto max-w-[80%]">
          <span className="text-xs font-bold text-muted-foreground">Thinking</span>
          <div className="rounded-lg p-3 bg-muted text-muted-foreground italic">
            {reasoningText}
          </div>
        </div>
      )}

      {/* Tool Progress 显示 */}
      {toolProgress.length > 0 && (
        <div className="mr-auto max-w-[80%]">
          <span className="text-xs font-bold text-muted-foreground">Tools</span>
          <div className="space-y-1 mt-1">
            {toolProgress.map((tool) => (
              <div key={tool.callId} className="text-sm flex items-center gap-2">
                <span className="font-mono">{tool.toolName}</span>
                <span className={cn(
                  "text-xs",
                  tool.status === "running" && "animate-pulse text-blue-500",
                  tool.status === "done" && "text-green-500",
                  tool.status === "error" && "text-red-500"
                )}>
                  {tool.status}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
