"use client";

import { PanelLeft, PanelRight } from "lucide-react";
import { useSidebarLeft } from "@/components/ui/sidebar";

export function SidebarToggle({ className }: { className?: string }) {
  const { leftState: state, toggleLeft } = useSidebarLeft();

  return (
    <button
      onClick={toggleLeft}
      className={className}
      aria-label={state === "expanded" ? "Collapse sidebar" : "Expand sidebar"}
    >
      {state === "expanded" ? (
        <PanelRight className="h-5 w-5" />
      ) : (
        <PanelLeft className="h-5 w-5" />
      )}
    </button>
  );
}
