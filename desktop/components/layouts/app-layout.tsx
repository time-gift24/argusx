"use client";

import { TooltipProvider } from "@/components/ui/tooltip";
import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  SidebarProvider,
  SidebarInset,
} from "@/components/ui/sidebar";
import { Toaster } from "@/components/ui/sonner";
import "../../app/globals.css";
import { AppSidebar } from "./sidebar/app-sidebar";
import { ModuleSidebar } from "./sidebar/module-sidebar";
import { SidebarTrigger } from "./sidebar/sidebar-trigger";
import { ThemeToggle } from "./theme-toggle";
import {
  Breadcrumb,
  BreadcrumbEllipsis,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

function RouteBreadcrumb({ pathname }: { pathname: string }) {
  if (pathname !== "/sop/annotation") {
    return null;
  }

  return (
    <Breadcrumb>
      <BreadcrumbList>
        <BreadcrumbItem>
          <BreadcrumbLink asChild>
            <Link href="/">工作台</Link>
          </BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
          <BreadcrumbLink asChild>
            <Link href="/sop/annotation">SOP</Link>
          </BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                aria-label="打开 SOP 页面导航"
                className="size-6 rounded-sm text-muted-foreground hover:text-foreground"
                size="icon-sm"
                type="button"
                variant="ghost"
              >
                <BreadcrumbEllipsis />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="w-44 min-w-44">
              <DropdownMenuLabel>SOP 页面</DropdownMenuLabel>
              <DropdownMenuItem asChild>
                <Link href="/sop/annotation">标注</Link>
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem disabled>更多页面待规划</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
          <BreadcrumbPage>标注</BreadcrumbPage>
        </BreadcrumbItem>
      </BreadcrumbList>
    </Breadcrumb>
  );
}

export function AppLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const isChatRoute =
    pathname === "/" || pathname === "/chat" || pathname.startsWith("/chat/");
  const showHeaderBreadcrumb = !isChatRoute;

  return (
    <TooltipProvider>
      <SidebarProvider
        className="h-svh overflow-hidden"
        defaultLeftOpen={true}
        defaultRightOpen={false}
      >
        <AppSidebar variant="inset" />
        <SidebarInset className="min-h-0">
          <header className="shrink-0 border-b px-5 py-3 xl:px-8">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2">
                <SidebarTrigger
                  className="-ml-1"
                  side="left"
                />
                <span className="text-sm text-muted-foreground">ArgusX</span>
              </div>
              <div className="flex items-center gap-2">
                <ThemeToggle />
                {!isChatRoute ? (
                  <SidebarTrigger
                    className="-mr-1"
                    side="right"
                  />
                ) : null}
              </div>
            </div>
            {showHeaderBreadcrumb ? (
              <div className="mt-2">
                <RouteBreadcrumb pathname={pathname} />
              </div>
            ) : null}
          </header>
          <div
            data-slot="main-scroll-region"
            className="min-h-0 flex-1 overflow-y-auto"
          >
            <div className="flex min-h-full flex-col gap-5 p-5 xl:px-8 xl:py-6">
              {children}
            </div>
          </div>
        </SidebarInset>
        {!isChatRoute ? <ModuleSidebar variant="sidebar" side="right" /> : null}
      </SidebarProvider>
      <Toaster position="top-right" richColors />
    </TooltipProvider>
  );
}
