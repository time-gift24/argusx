"use client";

import { useControllableState } from "@radix-ui/react-use-controllable-state";
import type { KeyboardEventHandler } from "react";
import { useEffect, useState } from "react";

import {
  AI_PROMPT_COMPOSER_STYLES,
  AI_RUNTIME_DENSITY,
} from "@/components/ai/styles";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";

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
  const hasAgents = agents.length > 0;
  const hasWorkflows = workflows.length > 0;
  const [category, setCategory] = useState<PromptComposerCategory>(
    hasAgents ? "agent" : "workflow"
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
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<"idle" | "submitting">("idle");
  const currentItems = category === "agent" ? agents : workflows;
  const currentSelectionId = lastSelectionByCategory[category];
  const currentSelection =
    currentItems.find((item) => item.id === currentSelectionId) ??
    currentItems[0] ??
    null;
  const isSubmitting = status === "submitting";

  useEffect(() => {
    setLastSelectionByCategory((previous) => {
      const nextAgent =
        agents.find((item) => item.id === previous.agent)?.id ?? agents[0]?.id ?? "";
      const nextWorkflow =
        workflows.find((item) => item.id === previous.workflow)?.id ??
        workflows[0]?.id ??
        "";

      if (nextAgent === previous.agent && nextWorkflow === previous.workflow) {
        return previous;
      }

      return {
        agent: nextAgent,
        workflow: nextWorkflow,
      };
    });

    setCategory((previous) => {
      if (previous === "agent" && agents.length > 0) {
        return previous;
      }
      if (previous === "workflow" && workflows.length > 0) {
        return previous;
      }
      if (agents.length > 0) {
        return "agent";
      }
      if (workflows.length > 0) {
        return "workflow";
      }
      return previous;
    });
  }, [agents, workflows]);

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
    if (isSubmitting) {
      return;
    }

    setLastSelectionByCategory((previous) => ({
      ...previous,
      [nextCategory]: selectionId,
    }));
    setCategory(nextCategory);
  };

  const submit = async () => {
    const nextDraft = draft.trim();

    if (!nextDraft || !currentSelection || isSubmitting) {
      return;
    }

    setStatus("submitting");
    setError(null);

    try {
      await onSubmit({
        category,
        draft: nextDraft,
        selectionId: currentSelection.id,
      });
      setDraft("");
    } catch {
      setError("Unable to send prompt. Try again.");
    } finally {
      setStatus("idle");
    }
  };

  const handleKeyDown: KeyboardEventHandler<HTMLTextAreaElement> = (event) => {
    if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) {
      return;
    }

    if (draft.trim().length === 0 || !currentSelection || isSubmitting) {
      return;
    }

    event.preventDefault();
    void submit();
  };

  return (
    <div
      className={cn(
        AI_PROMPT_COMPOSER_STYLES.root,
        "overflow-hidden",
        AI_RUNTIME_DENSITY.blockGap
      )}
      data-slot="prompt-composer"
    >
      <textarea
        aria-label="Prompt"
        className={AI_PROMPT_COMPOSER_STYLES.textarea}
        disabled={isSubmitting}
        name="prompt"
        onChange={(event) => setDraft(event.currentTarget.value)}
        onKeyDown={handleKeyDown}
        placeholder="Tell the selected agent what to do next"
        value={draft}
      />
      {error ? (
        <p className={AI_PROMPT_COMPOSER_STYLES.errorText} role="alert">
          {error}
        </p>
      ) : null}
      <div
        className={AI_PROMPT_COMPOSER_STYLES.modeBar}
        data-slot="prompt-composer-mode-bar"
      >
        {hasAgents ? (
          <button
            className={cn(
              AI_PROMPT_COMPOSER_STYLES.categoryBase,
              AI_PROMPT_COMPOSER_STYLES.category.agent
            )}
            data-category="agent"
            data-state={category === "agent" ? "active" : "inactive"}
            disabled={isSubmitting}
            onClick={() => handleCategoryChange("agent")}
            type="button"
          >
            Agents
          </button>
        ) : null}
        {hasWorkflows ? (
          <button
            className={cn(
              AI_PROMPT_COMPOSER_STYLES.categoryBase,
              AI_PROMPT_COMPOSER_STYLES.category.workflow
            )}
            data-category="workflow"
            data-state={category === "workflow" ? "active" : "inactive"}
            disabled={isSubmitting}
            onClick={() => handleCategoryChange("workflow")}
            type="button"
          >
            Workflows
          </button>
        ) : null}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button
              className={cn(
                AI_PROMPT_COMPOSER_STYLES.selectionTrigger,
                "max-w-40 truncate"
              )}
              disabled={isSubmitting}
              type="button"
            >
              {currentSelection?.label}
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            {hasAgents ? (
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
            ) : null}
            {hasAgents && hasWorkflows ? <DropdownMenuSeparator /> : null}
            {hasWorkflows ? (
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
            ) : null}
          </DropdownMenuContent>
        </DropdownMenu>
        <p className={cn(AI_PROMPT_COMPOSER_STYLES.selectionDescription, "flex-1")}>
          {currentSelection?.description}
        </p>
        <button
          className={AI_PROMPT_COMPOSER_STYLES.submitButton}
          disabled={draft.trim().length === 0 || isSubmitting}
          onClick={() => void submit()}
          type="button"
        >
          {isSubmitting ? <Spinner className="size-3.5" /> : null}
          Send
        </button>
      </div>
    </div>
  );
}
