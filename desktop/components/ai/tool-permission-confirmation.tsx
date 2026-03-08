"use client";

import {
  Confirmation,
  ConfirmationAccepted,
  ConfirmationAction,
  ConfirmationActions,
  ConfirmationRejected,
  ConfirmationRequest,
  ConfirmationTitle,
} from "@/components/ai-elements/confirmation";
import { cn } from "@/lib/utils";

export type ToolPermissionConfirmationState =
  | "requested"
  | "accepted"
  | "rejected";

type ToolPermissionConfirmationProps = {
  argumentsSummary?: string;
  className?: string;
  error?: string | null;
  isSubmitting?: boolean;
  onAllow?: () => void;
  onDeny?: () => void;
  requestId: string;
  state: ToolPermissionConfirmationState;
  toolName: string;
};

export function ToolPermissionConfirmation({
  argumentsSummary,
  className,
  error,
  isSubmitting = false,
  onAllow,
  onDeny,
  requestId,
  state,
  toolName,
}: ToolPermissionConfirmationProps) {
  return (
    <Confirmation
      approval={approvalFromState(requestId, state)}
      className={cn(
        "rounded-3xl border border-border/70 bg-background/95 p-4 shadow-lg backdrop-blur supports-[backdrop-filter]:bg-background/80",
        className
      )}
      data-slot="tool-permission-confirmation"
      data-state={state}
      state={state === "requested" ? "approval-requested" : "approval-responded"}
    >
      <ConfirmationRequest>
        <div className="space-y-2">
          <ConfirmationTitle className="block text-sm font-medium text-foreground">
            Tool permission required.
          </ConfirmationTitle>
          <p className="text-sm text-foreground">{toolName}</p>
          {argumentsSummary ? (
            <p className="break-all text-xs text-muted-foreground">
              {argumentsSummary}
            </p>
          ) : null}
          {error ? (
            <p className="text-xs text-destructive" role="alert">
              {error}
            </p>
          ) : null}
        </div>
        <ConfirmationActions className="mt-2">
          <ConfirmationAction
            disabled={isSubmitting}
            onClick={onDeny}
            variant="outline"
          >
            Deny
          </ConfirmationAction>
          <ConfirmationAction disabled={isSubmitting} onClick={onAllow}>
            Allow
          </ConfirmationAction>
        </ConfirmationActions>
      </ConfirmationRequest>

      <ConfirmationAccepted>
        <div className="space-y-1">
          <ConfirmationTitle className="block text-sm font-medium text-foreground">
            Tool request approved.
          </ConfirmationTitle>
          <p className="text-xs text-muted-foreground">{toolName}</p>
        </div>
      </ConfirmationAccepted>

      <ConfirmationRejected>
        <div className="space-y-1">
          <ConfirmationTitle className="block text-sm font-medium text-foreground">
            Tool request denied.
          </ConfirmationTitle>
          <p className="text-xs text-muted-foreground">{toolName}</p>
        </div>
      </ConfirmationRejected>
    </Confirmation>
  );
}

function approvalFromState(
  requestId: string,
  state: ToolPermissionConfirmationState
) {
  switch (state) {
    case "accepted":
      return { approved: true as const, id: requestId };
    case "rejected":
      return { approved: false as const, id: requestId };
    default:
      return { id: requestId };
  }
}
