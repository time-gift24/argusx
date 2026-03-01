"use client";

import { MessageCircle, Plus } from "lucide-react";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarInput,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";

const chatHistory = [
  {
    title: "New Chat",
    url: "/chat",
    icon: Plus,
    isActive: true,
  },
  {
    title: "Project Discussion",
    url: "/chat/project-discussion",
    icon: MessageCircle,
  },
  {
    title: "Code Review",
    url: "/chat/code-review",
    icon: MessageCircle,
  },
  {
    title: "Bug Analysis",
    url: "/chat/bug-analysis",
    icon: MessageCircle,
  },
];

import { cn } from "@/lib/utils";

export function ChatSidebar({ className, ...props }: React.ComponentProps<typeof Sidebar>) {
  return (
    <Sidebar
      {...props}
      side="right"
      className={cn(className)}
    >
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>Chat</SidebarGroupLabel>
          <SidebarGroupContent>
            <form>
              <SidebarInput placeholder="Search chats..." type="search" />
            </form>
          </SidebarGroupContent>
        </SidebarGroup>
        <SidebarGroup>
          <SidebarGroupLabel>Conversations</SidebarGroupLabel>
          <SidebarMenu>
            {chatHistory.map((item) => (
              <SidebarMenuItem key={item.title}>
                <SidebarMenuButton asChild isActive={item.isActive}>
                  <a href={item.url}>
                    <item.icon className="h-4 w-4" />
                    <span>{item.title}</span>
                  </a>
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroup>
      </SidebarContent>
      <SidebarRail side="right" />
    </Sidebar>
  );
}
