"use client";

import { ChatModulePlaceholder } from "@/components/placeholders/chat-module-placeholder";
import { cn } from "@/lib/utils";
import {
  Sidebar,
  SidebarContent,
  SidebarRail,
} from "@/components/ui/sidebar";

export function ModuleSidebar({ className, ...props }: React.ComponentProps<typeof Sidebar>) {
  return (
    <Sidebar
      {...props}
      side="right"
      className={cn("overflow-hidden group-data-[side=right]:!border-l-0", className)}
    >
      <SidebarContent className="min-h-0 p-0">
        <div className="flex min-h-0 flex-1">
          <ChatModulePlaceholder variant="sidebar" />
        </div>
      </SidebarContent>
      <SidebarRail side="right" />
    </Sidebar>
  );
}
