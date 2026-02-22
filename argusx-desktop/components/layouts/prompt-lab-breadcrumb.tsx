"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { ChevronDown } from "lucide-react";

const promptLabPages = [
  { title: "Dashboard", href: "/prompt-lab" },
  { title: "Checklist", href: "/prompt-lab/checklist" },
  { title: "Golden Sets", href: "/prompt-lab/golden-sets" },
  { title: "Results", href: "/prompt-lab/results" },
  { title: "Logs", href: "/prompt-lab/logs" },
];

function getPageTitle(pathname: string): string {
  const page = promptLabPages.find((p) => p.href === pathname);
  return page?.title || "PromptLab";
}

function isPromptLabSubPage(pathname: string): boolean {
  return pathname.startsWith("/prompt-lab") && pathname !== "/prompt-lab";
}

export function PromptLabBreadcrumb() {
  const pathname = usePathname();

  if (!isPromptLabSubPage(pathname)) {
    return null;
  }

  const currentTitle = getPageTitle(pathname);
  const otherPages = promptLabPages.filter((p) => p.href !== pathname);

  return (
    <Breadcrumb>
      <BreadcrumbList>
        <BreadcrumbItem>
          <BreadcrumbLink asChild>
            <Link href="/">首页</Link>
          </BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
          <DropdownMenu>
            <DropdownMenuTrigger className="flex items-center gap-1 hover:text-foreground">
              <span>PromptLab</span>
              <ChevronDown className="h-3 w-3" />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              {otherPages.map((page) => (
                <DropdownMenuItem key={page.href} asChild>
                  <Link href={page.href}>{page.title}</Link>
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
          <BreadcrumbPage>{currentTitle}</BreadcrumbPage>
        </BreadcrumbItem>
      </BreadcrumbList>
    </Breadcrumb>
  );
}
