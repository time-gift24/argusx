"use client";

import { PromptComposer } from "@/components/ai";

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

const WORKFLOWS = [
  {
    description: "Draft a design-oriented implementation brief",
    id: "design",
    label: "Write Design",
  },
  {
    description: "Prepare the task for a focused implementation pass",
    id: "execute",
    label: "Execute Plan",
  },
] as const;

export default function ChatPage() {
  return (
    <div className="flex min-h-0 flex-1 flex-col p-4 lg:p-6">
      <div className="flex-1" />
      <div className="mx-auto w-full max-w-5xl">
        <PromptComposer
          agents={[...AGENTS]}
          onSubmit={async () => {}}
          workflows={[...WORKFLOWS]}
        />
      </div>
    </div>
  );
}
