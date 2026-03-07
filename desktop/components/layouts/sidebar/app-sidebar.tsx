"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { FlaskConical, MessageSquare, NotebookPen } from "lucide-react";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";

const navMain = [
  {
    title: "对话",
    url: "/chat",
    icon: MessageSquare,
  },
  {
    title: "SOP 标注",
    url: "/sop/annotation",
    icon: NotebookPen,
  },
];

const navDev = [
  {
    title: "Dev",
    url: "/dev",
    icon: FlaskConical,
  },
];

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  const pathname = usePathname();
  const isChatActive = pathname === "/" || pathname === "/chat" || pathname.startsWith("/chat/");
  const isDevActive = pathname === "/dev" || pathname.startsWith("/dev/");

  return (
    <Sidebar {...props}>
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>工作区</SidebarGroupLabel>
          <SidebarMenu>
            {navMain.map((item) => (
              <SidebarMenuItem key={item.title}>
                <SidebarMenuButton
                  asChild
                  isActive={item.url === "/chat" ? isChatActive : pathname === item.url}
                >
                  <Link href={item.url}>
                    <item.icon className="h-4 w-4" />
                    <span>{item.title}</span>
                  </Link>
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroup>
        <SidebarGroup>
          <SidebarGroupLabel>Dev</SidebarGroupLabel>
          <SidebarMenu>
            {navDev.map((item) => (
              <SidebarMenuItem key={item.title}>
                <SidebarMenuButton asChild isActive={isDevActive}>
                  <Link href={item.url}>
                    <item.icon className="h-4 w-4" />
                    <span>{item.title}</span>
                  </Link>
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroup>
      </SidebarContent>
      <SidebarRail />
    </Sidebar>
  );
}
