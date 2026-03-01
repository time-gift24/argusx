"use client";

import { PanelLeft, PanelRight } from "lucide-react";
import {
  useSidebarLeft,
  useSidebarMobile,
  useSidebarRight,
} from "@/components/ui/sidebar";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface SidebarTriggerProps {
  className?: string;
  onClick?: () => void;
  isActive?: boolean;
  side?: "left" | "right";
}

export function SidebarTrigger({
  className,
  onClick,
  isActive,
  side = "left",
}: SidebarTriggerProps) {
  const { isMobile } = useSidebarMobile();
  const { leftState, toggleLeft } = useSidebarLeft();
  const { rightState, toggleRight } = useSidebarRight();

  if (isMobile) {
    return null;
  }

  const state = side === "left" ? leftState : rightState;

  const handleClick = () => {
    if (side === "left") {
      toggleLeft();
    } else {
      toggleRight();
    }
    onClick?.();
  };

  return (
    <Button
      variant="ghost"
      size="icon"
      className={cn("h-9 w-9", className)}
      onClick={handleClick}
      aria-pressed={isActive ?? state === "expanded"}
    >
      {side === "left" ? (
        <PanelLeft className="h-4 w-4" />
      ) : (
        <PanelRight className="h-4 w-4" />
      )}
      <span className="sr-only">
        {side === "left" ? "Toggle sidebar" : "Toggle chat panel"}
      </span>
    </Button>
  );
}
