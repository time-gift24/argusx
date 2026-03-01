"use client";

import { useState } from "react";
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { ChatSession } from "@/lib/stores/chat-store";

interface BadgeContextMenuProps {
  session: ChatSession;
  children: React.ReactNode;
  onRename: (title: string) => void;
  onChangeColor: (color: string) => void;
  onDelete: () => void;
}

const COLORS = [
  { value: "chart-1", label: "Blue" },
  { value: "chart-2", label: "Cyan" },
  { value: "chart-3", label: "Teal" },
  { value: "chart-4", label: "Indigo" },
  { value: "chart-5", label: "Violet" },
];

export function BadgeContextMenu({
  session,
  children,
  onRename,
  onChangeColor,
  onDelete,
}: BadgeContextMenuProps) {
  const [showRenameDialog, setShowRenameDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [newTitle, setNewTitle] = useState(session.title);

  const handleRename = () => {
    if (newTitle.trim()) {
      onRename(newTitle.trim());
      setShowRenameDialog(false);
    }
  };

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>{children}</DropdownMenuTrigger>
        <DropdownMenuContent className="w-48">
          <DropdownMenuItem onClick={() => setShowRenameDialog(true)}>
            Rename
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          {COLORS.map((color) => (
            <DropdownMenuItem
              key={color.value}
              onClick={() => onChangeColor(color.value)}
            >
              <span className={`mr-2 size-3 rounded-full bg-${color.value}`} />
              {color.label}
            </DropdownMenuItem>
          ))}
          <DropdownMenuSeparator />
          <DropdownMenuItem
            className="text-destructive"
            onClick={() => setShowDeleteDialog(true)}
          >
            Delete
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      {/* Rename Dialog */}
      <Dialog open={showRenameDialog} onOpenChange={setShowRenameDialog}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Rename Session</DialogTitle>
          </DialogHeader>
          <div className="py-4">
            <Label htmlFor="title">Title</Label>
            <Input
              className="mt-2"
              id="title"
              onChange={(e) => setNewTitle(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleRename()}
              value={newTitle}
            />
          </div>
          <DialogFooter>
            <Button onClick={handleRename} size="sm">
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Session?</AlertDialogTitle>
            <AlertDialogDescription>
              This will permanently delete &quot;{session.title}&quot; and all its
              messages. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground"
              onClick={onDelete}
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
