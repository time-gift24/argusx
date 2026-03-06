"use client";

import { useEffect } from "react";
import Link from "next/link";
import { AnnotationWorkspace } from "./annotation-workspace";
import { useAnnotationStore } from "@/lib/stores/annotation-store";
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

export function AnnotationPage() {
  const loadCatalog = useAnnotationStore((store) => store.loadCatalog);

  useEffect(() => {
    loadCatalog();
  }, [loadCatalog]);

  return (
    <div className="flex flex-col gap-4">
      <header className="space-y-3">
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
        <h1 className="text-xl font-semibold">SOP 标注工作台</h1>
        <p className="text-sm text-muted-foreground">
          左侧浏览 SOP 审核文本，右侧维护单条标注。
        </p>
      </header>
      <AnnotationWorkspace />
    </div>
  );
}
