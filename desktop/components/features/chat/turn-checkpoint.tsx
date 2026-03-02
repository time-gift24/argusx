"use client";

import type { AgentTurnVM } from "@/lib/stores/chat-store";

import {
  Checkpoint,
  CheckpointIcon,
  CheckpointTrigger,
} from "@/components/ai-elements/checkpoint";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { restoreTurnCheckpoint } from "@/lib/api/chat";
import { useChatStore } from "@/lib/stores/chat-store";
import { Loader2Icon } from "lucide-react";
import { useState } from "react";

interface TurnCheckpointProps {
  sessionId: string;
  turn: AgentTurnVM;
}

export function TurnCheckpoint({ sessionId, turn }: TurnCheckpointProps) {
  const restoreToCheckpoint = useChatStore((state) => state.restoreToCheckpoint);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [isRestoring, setIsRestoring] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (turn.status !== "done") {
    return null;
  }

  const handleRestore = async (event: React.MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    if (isRestoring) {
      return;
    }
    setIsRestoring(true);
    setError(null);
    try {
      const result = await restoreTurnCheckpoint({
        sessionId,
        turnId: turn.id,
      });
      restoreToCheckpoint(sessionId, result.restoredTurnId, result.removedTurnIds);
      setDialogOpen(false);
    } catch (restoreError) {
      const message =
        restoreError instanceof Error
          ? restoreError.message
          : "Failed to restore checkpoint";
      setError(message);
    } finally {
      setIsRestoring(false);
    }
  };

  return (
    <div className="w-full">
      <Checkpoint className="text-[11px]">
        <CheckpointIcon className="size-3.5" />
        <AlertDialog onOpenChange={setDialogOpen} open={dialogOpen}>
          <CheckpointTrigger
            className="h-6 px-2 text-[11px]"
            disabled={isRestoring}
            onClick={() => setDialogOpen(true)}
            size="sm"
            tooltip="Restore chat to this checkpoint"
          >
            {isRestoring ? (
              <span className="inline-flex items-center gap-1.5">
                <Loader2Icon className="size-3 animate-spin" />
                Restoring...
              </span>
            ) : (
              "Restore checkpoint"
            )}
          </CheckpointTrigger>
          <AlertDialogContent size="sm">
            <AlertDialogHeader>
              <AlertDialogTitle>Restore checkpoint?</AlertDialogTitle>
              <AlertDialogDescription>
                This will permanently remove chat turns after this checkpoint.
                This action cannot be undone.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={isRestoring}>Cancel</AlertDialogCancel>
              <AlertDialogAction
                className="bg-destructive text-destructive-foreground"
                disabled={isRestoring}
                onClick={handleRestore}
              >
                {isRestoring ? "Restoring..." : "Restore"}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </Checkpoint>
      {error ? (
        <p className="mt-1 text-[11px] text-destructive">{error}</p>
      ) : null}
    </div>
  );
}
