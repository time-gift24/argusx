"use client";

import { useChatStore } from "@/lib/stores/chat-store";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";

const colorClasses: Record<string, string> = {
  blue: "border-blue-500 text-blue-500",
  green: "border-green-500 text-green-500",
  purple: "border-purple-500 text-purple-500",
  orange: "border-orange-500 text-orange-500",
  pink: "border-pink-500 text-pink-500",
  cyan: "border-cyan-500 text-cyan-500",
};

export function SessionSwitcher() {
  const { sessions, currentSessionId, createSession, switchSession } = useChatStore();

  const handleCreate = async () => {
    await createSession("New Chat");
  };

  const handleSwitch = async (id: string) => {
    await switchSession(id);
  };

  return (
    <div className="flex items-center gap-2 px-4 py-2 border-t">
      {sessions.map((s) => (
        <Badge
          key={s.id}
          variant={s.id === currentSessionId ? "default" : "outline"}
          className={`cursor-pointer ${colorClasses[s.color] || "border-gray-500 text-gray-500"}`}
          onClick={() => handleSwitch(s.id)}
        >
          <span className={`w-2 h-2 rounded-full bg-current mr-1`} />
          {s.title}
        </Badge>
      ))}
      <Button variant="ghost" size="sm" onClick={handleCreate}>
        <Plus className="w-4 h-4" />
      </Button>
    </div>
  );
}
