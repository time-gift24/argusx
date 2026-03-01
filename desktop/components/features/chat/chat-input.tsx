"use client";

import { useState, useRef, useEffect } from "react";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { Send } from "lucide-react";
import { useChatStore } from "@/lib/stores/chat-store";

export function ChatInput() {
  const [input, setInput] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const { sendMessage, agentStatus } = useChatStore();

  const handleSend = async () => {
    if (!input.trim() || agentStatus === "running") return;
    const content = input.trim();
    setInput("");
    await sendMessage(content);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

  return (
    <div className="border-t p-4 flex gap-2 items-end">
      <Textarea
        ref={textareaRef}
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="输入消息... (Shift+Enter 换行)"
        className="flex-1 min-h-[60px] max-h-[200px] resize-none"
        disabled={agentStatus === "running"}
      />
      <Button
        onClick={handleSend}
        disabled={!input.trim() || agentStatus === "running"}
      >
        <Send className="w-4 h-4" />
      </Button>
    </div>
  );
}
