"use client";

import { useControllableState } from "@radix-ui/react-use-controllable-state";
import type { KeyboardEventHandler } from "react";

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
  const category: PromptComposerCategory =
    agents.length > 0 ? "agent" : "workflow";
  const currentSelection = agents[0] ?? workflows[0] ?? null;
  const [draft, setDraft] = useControllableState({
    defaultProp: defaultValue ?? "",
    onChange: onValueChange,
    prop: value,
  });

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
        <button type="button">Agents</button>
        <button type="button">Workflows</button>
        <button type="button">{currentSelection?.label}</button>
        <button disabled={draft.trim().length === 0} type="submit">
          Send
        </button>
      </div>
    </div>
  );
}
