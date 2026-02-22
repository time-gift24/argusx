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
import { PromptLabBreadcrumb } from "./prompt-lab-breadcrumb";

export function AppLayout({ children }: { children: React.ReactNode }) {
  return (
    <TooltipProvider>
      <SidebarProvider defaultLeftOpen={true} defaultRightOpen={false}>
        <AppSidebar variant="floating" />
        <SidebarInset>
          <header className="flex h-16 shrink-0 items-center justify-between gap-2 border-b px-4">
            <div className="w-20">
              <SidebarTrigger
                className="-ml-1"
                side="left"
              />
            </div>
            <PromptLabBreadcrumb />
            <div className="flex items-center gap-2 w-20 justify-end">
              <ThemeToggle />
              <SidebarTrigger
                className="-mr-1"
                side="right"
              />
            </div>
          </header>
          <div className="flex flex-1 flex-col gap-4 p-4">
            {children}
          </div>
        </SidebarInset>
        <ChatSidebar variant="floating" side="right" />
      </SidebarProvider>
    </TooltipProvider>
  );
}
