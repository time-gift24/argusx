"use client";

import { useControllableState } from "@radix-ui/react-use-controllable-state";
import type { KeyboardEventHandler } from "react";
import { useState } from "react";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

export type PromptComposerCategory = "agent" | "workflow";

export type PromptComposerOption = {
  id: string;
  label: string;
  description: string;
  disabled?: boolean;
};

export type PromptComposerSubmitPayload = {
  category: PromptComposerCategory;
  selectionId: string;
  draft: string;
};

export type PromptComposerProps = {
  agents: PromptComposerOption[];
  workflows: PromptComposerOption[];
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  onSubmit: (
    payload: PromptComposerSubmitPayload
  ) => void | Promise<void>;
};

export function PromptComposer({
  agents,
  defaultValue,
  onSubmit,
  onValueChange,
  value,
  workflows,
}: PromptComposerProps) {
  const [category, setCategory] = useState<PromptComposerCategory>(
    agents.length > 0 ? "agent" : "workflow"
  );
  const [lastSelectionByCategory, setLastSelectionByCategory] = useState<{
    agent: string;
    workflow: string;
  }>({
    agent: agents[0]?.id ?? "",
    workflow: workflows[0]?.id ?? "",
  });
  const [draft, setDraft] = useControllableState({
    defaultProp: defaultValue ?? "",
    onChange: onValueChange,
    prop: value,
  });
  const currentItems = category === "agent" ? agents : workflows;
  const currentSelectionId = lastSelectionByCategory[category];
  const currentSelection =
    currentItems.find((item) => item.id === currentSelectionId) ??
    currentItems[0] ??
    null;

  const handleCategoryChange = (nextCategory: PromptComposerCategory) => {
    const nextItems = nextCategory === "agent" ? agents : workflows;

    if (nextItems.length === 0) {
      return;
    }

    setCategory(nextCategory);
  };

  const handleSelectionChange = (
    nextCategory: PromptComposerCategory,
    selectionId: string
  ) => {
    setLastSelectionByCategory((previous) => ({
      ...previous,
      [nextCategory]: selectionId,
    }));
    setCategory(nextCategory);
  };

  const handleKeyDown: KeyboardEventHandler<HTMLTextAreaElement> = (event) => {
    if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) {
      return;
    }

    const nextDraft = draft.trim();

    if (!nextDraft || !currentSelection) {
      return;
    }

    event.preventDefault();
    void onSubmit({
      category,
      draft: nextDraft,
      selectionId: currentSelection.id,
    });
  };

  return (
    <div data-slot="prompt-composer">
      <textarea
        aria-label="Prompt"
        name="prompt"
        onChange={(event) => setDraft(event.currentTarget.value)}
        onKeyDown={handleKeyDown}
        value={draft}
      />
      <div>
        <button onClick={() => handleCategoryChange("agent")} type="button">
          Agents
        </button>
        <button onClick={() => handleCategoryChange("workflow")} type="button">
          Workflows
        </button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button type="button">{currentSelection?.label}</button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            <DropdownMenuGroup>
              <DropdownMenuLabel>Agents</DropdownMenuLabel>
              {agents.map((item) => (
                <DropdownMenuItem
                  disabled={item.disabled}
                  key={item.id}
                  onClick={() => handleSelectionChange("agent", item.id)}
                >
                  {item.label}
                </DropdownMenuItem>
              ))}
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuGroup>
              <DropdownMenuLabel>Workflows</DropdownMenuLabel>
              {workflows.map((item) => (
                <DropdownMenuItem
                  disabled={item.disabled}
                  key={item.id}
                  onClick={() => handleSelectionChange("workflow", item.id)}
                >
                  {item.label}
                </DropdownMenuItem>
              ))}
            </DropdownMenuGroup>
          </DropdownMenuContent>
        </DropdownMenu>
        <button disabled={draft.trim().length === 0} type="submit">
          Send
        </button>
      </div>
    </div>
  );
}
