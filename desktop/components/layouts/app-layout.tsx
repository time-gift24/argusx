"use client";

import { TooltipProvider } from "@/components/ui/tooltip";
import {
  SidebarProvider,
  SidebarInset,
} from "@/components/ui/sidebar";
import "../../app/globals.css";
import { AppSidebar } from "./sidebar/app-sidebar";
import { ChatSidebar } from "./sidebar/chat-sidebar";
import { SidebarTrigger } from "./sidebar/sidebar-trigger";
import { ThemeToggle } from "./theme-toggle";

export function AppLayout({ children }: { children: React.ReactNode }) {
  return (
    <TooltipProvider>
      <SidebarProvider
        className="h-svh overflow-hidden"
        defaultLeftOpen={true}
        defaultRightOpen={false}
      >
        <AppSidebar variant="floating" />
        <SidebarInset className="min-h-0">
          <header className="flex h-16 shrink-0 items-center justify-between gap-2 border-b px-4">
            <div className="flex items-center gap-2">
              <SidebarTrigger
                className="-ml-1"
                side="left"
              />
              <span className="text-sm text-muted-foreground">ArgusX</span>
            </div>
            <div className="flex items-center gap-2">
              <ThemeToggle />
              <SidebarTrigger
                className="-mr-1"
                side="right"
              />
            </div>
          </header>
          <div className="flex min-h-0 flex-1 flex-col gap-4 p-4">
            {children}
          </div>
        </SidebarInset>
        <ChatSidebar variant="floating" side="right" />
      </SidebarProvider>
    </TooltipProvider>
  );
}
