export const AI_RUNTIME_DENSITY = {
  blockGap: "gap-1",
  bodyText: "text-[14px] leading-5",
  contentIndent: "pl-5",
  eyebrowText: "text-[12px] uppercase tracking-[0.08em] leading-[14px]",
  inlineCode:
    "rounded-sm border border-border/60 bg-muted/40 px-1.5 py-0.5 font-mono text-[12px] leading-[14px] text-foreground",
  mutedText: "text-[14px] leading-5 text-muted-foreground",
  sectionGap: "gap-1.5",
  triggerText: "text-[14px] leading-5",
} as const;

export const AI_STREAMDOWN_CLASSNAME = "ai-streamdown";

export const AI_CODE_SURFACE_CLASSNAME =
  "group relative overflow-hidden rounded-lg border border-border/60 bg-background text-foreground shadow-sm";

export const AI_PROMPT_COMPOSER_STYLES = {
  root:
    "rounded-2xl border border-border/70 bg-background/80 text-card-foreground shadow-sm backdrop-blur-sm",
  textarea:
    "min-h-24 w-full resize-none border-0 bg-transparent px-4 py-4 text-sm leading-6 outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-70",
  modeBar:
    "flex items-center gap-2 border-t border-border/60 px-3 py-2",
  categoryBase:
    "rounded-full border border-border/60 px-2.5 py-1 text-[11px] font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-60",
  category: {
    agent:
      "text-[color:var(--ai-composer-agent-foreground)] data-[state=active]:bg-[color:var(--ai-composer-agent-bg)] data-[state=active]:border-[color:var(--ai-composer-agent-border)]",
    workflow:
      "text-[color:var(--ai-composer-workflow-foreground)] data-[state=active]:bg-[color:var(--ai-composer-workflow-bg)] data-[state=active]:border-[color:var(--ai-composer-workflow-border)]",
  },
  selectionTrigger:
    "min-w-0 rounded-full border border-border/60 px-3 py-1 text-left text-xs text-foreground transition-colors hover:bg-muted/60 disabled:cursor-not-allowed disabled:opacity-60",
  selectionDescription: "min-w-0 truncate text-xs text-muted-foreground",
  submitButton:
    "inline-flex items-center gap-2 rounded-full bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors disabled:cursor-not-allowed disabled:opacity-60",
  errorText: "px-4 pb-3 text-xs text-destructive",
} as const;
