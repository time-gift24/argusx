"use client";

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
  onSubmit: (
    payload: PromptComposerSubmitPayload
  ) => void | Promise<void>;
};

export function PromptComposer({
  agents,
  onSubmit: _onSubmit,
  workflows,
}: PromptComposerProps) {
  const currentSelection = agents[0] ?? workflows[0] ?? null;

  return (
    <div data-slot="prompt-composer">
      <textarea aria-label="Prompt" name="prompt" />
      <div>
        <button type="button">Agents</button>
        <button type="button">Workflows</button>
        <button type="button">{currentSelection?.label}</button>
        <button disabled type="submit">
          Send
        </button>
      </div>
    </div>
  );
}
