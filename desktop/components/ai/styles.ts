import { cn } from "@/lib/utils";

export const CHAT_STYLES = {
  runtimeSurface: {
    base: "llm-chat-runtime-surface rounded-xl border",
    variant: {
      code: "bg-[var(--chat-runtime-surface-bg)] border-[var(--chat-runtime-surface-border)]",
      process: "bg-[var(--chat-runtime-surface-bg)] border-[var(--chat-runtime-surface-border)]",
    },
  },
} as const;

export type RuntimeSurfaceVariant = keyof typeof CHAT_STYLES.runtimeSurface.variant;

// Helper function to get runtime surface class
export function getRuntimeSurfaceClass(variant: RuntimeSurfaceVariant = "code") {
  return cn(
    CHAT_STYLES.runtimeSurface.base,
    CHAT_STYLES.runtimeSurface.variant[variant]
  );
}
