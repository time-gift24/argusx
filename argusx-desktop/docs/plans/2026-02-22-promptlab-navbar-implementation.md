# PromptLab 导航实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** 在 Sidebar 中为 PromptLab 添加可折叠子菜单，在各子页面头部添加面包屑导航

**Architecture:** 使用现有的 shadcn/ui 组件（Collapsible, DropdownMenu, Breadcrumb）实现两级导航

**Tech Stack:** Next.js 16, React, Tailwind CSS v4, shadcn/ui (radix-ui)

---

## Task 1: 修改 AppSidebar 添加 Collapsible 子菜单

**Files:**
- Modify: `components/layouts/sidebar/app-sidebar.tsx`

**Step 1: 添加 Collapsible 和 DropdownMenu 导入**

```tsx
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
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { ChevronDown, ClipboardCheck } from "lucide-react";
```

**Step 2: 定义 PromptLab 子菜单数据**

```tsx
const navPromptLab = [
  { title: "Dashboard", url: "/prompt-lab" },
  { title: "Checklist", url: "/prompt-lab/checklist" },
  { title: "Golden Sets", url: "/prompt-lab/golden-sets" },
  { title: "Results", url: "/prompt-lab/results" },
  { title: "Logs", url: "/prompt-lab/logs" },
];
```

**Step 3: 修改 navMain 定义，只保留 Dashboard**

```tsx
const navMain = [
  {
    title: "Dashboard",
    url: "/",
    icon: Home,
  },
];
```

**Step 4: 添加 Collapsible + DropdownMenu 实现**

在 navMain.map 之后添加：

```tsx
<SidebarMenuItem>
  <Collapsible defaultOpen={false} className="group/collapsible">
    <CollapsibleTrigger asChild>
      <SidebarMenuButton>
        <ClipboardCheck className="h-4 w-4" />
        <span>PromptLab</span>
        <ChevronDown className="ml-auto h-4 w-4 transition-transform group-data-[state=open]/collapsible:rotate-180" />
      </SidebarMenuButton>
    </CollapsibleTrigger>
    <CollapsibleContent>
      <SidebarMenu>
        {navPromptLab.map((item) => (
          <SidebarMenuItem key={item.title}>
            <SidebarMenuButton asChild isActive={pathname === item.url}>
              <Link href={item.url}>
                <span>{item.title}</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        ))}
      </SidebarMenu>
    </CollapsibleContent>
  </Collapsible>
</SidebarMenuItem>
```

**Step 5: 验证 Sidebar 渲染**

运行: `pnpm dev`
期望: Sidebar 显示 Dashboard 和可折叠的 PromptLab，展开后显示 5 个子菜单项

---

## Task 2: 创建 PromptLab 面包屑组件

**Files:**
- Create: `components/layouts/prompt-lab-breadcrumb.tsx`

**Step 1: 创建面包屑组件**

```tsx
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
```

**Step 2: 验证组件导出**

检查: `components/layouts/index.ts` (如果存在) 或直接在需要的地方导入

---

## Task 3: 修改 prompt-lab Dashboard 页面

**Files:**
- Modify: `app/prompt-lab/page.tsx`

**Step 1: 导入面包屑组件**

```tsx
import { PromptLabBreadcrumb } from "@/components/layouts/prompt-lab-breadcrumb";
```

**Step 2: 在页面顶部添加面包屑**

```tsx
return (
  <div className="space-y-6">
    <PromptLabBreadcrumb />
    <h1 className="text-2xl font-bold">PromptLab Dashboard</h1>
    ...
  </div>
);
```

---

## Task 4: 修改 checklist 页面

**Files:**
- Modify: `app/prompt-lab/checklist/page.tsx`

**Step 1: 导入面包屑组件**

```tsx
import { PromptLabBreadcrumb } from "@/components/layouts/prompt-lab-breadcrumb";
```

**Step 2: 在页面顶部添加面包屑**

在 return 的最外层 div 第一行添加 `<PromptLabBreadcrumb />`

---

## Task 5: 修改 golden-sets 页面

**Files:**
- Modify: `app/prompt-lab/golden-sets/page.tsx`

**Step 1: 读取并修改页面**

参考 Task 4 步骤

---

## Task 6: 修改 results 页面

**Files:**
- Modify: `app/prompt-lab/results/page.tsx`

**Step 1: 读取并修改页面**

参考 Task 4 步骤

---

## Task 7: 修改 logs 页面

**Files:**
- Modify: `app/prompt-lab/logs/page.tsx`

**Step 1: 读取并修改页面**

参考 Task 4 步骤

---

## Task 8: 验证完整功能

**Step 1: 运行开发服务器**

```bash
pnpm dev
```

**Step 2: 验证 Sidebar**

- 访问 `http://localhost:3000`
- 确认左侧 Sidebar 显示 Dashboard 和可折叠的 PromptLab
- 点击 PromptLab 确认展开显示 5 个子菜单项
- 点击各子菜单确认跳转正确

**Step 3: 验证面包屑**

- 访问各子页面 `/prompt-lab/checklist`, `/prompt-lab/golden-sets`, `/prompt-lab/results`, `/prompt-lab/logs`
- 确认显示 `首页 > PromptLab > [当前页面]` 格式
- 确认点击 PromptLab 位置的 Dropdown 可切换到其他页面

**Step 4: 提交代码**

```bash
git add components/layouts/sidebar/app-sidebar.tsx components/layouts/prompt-lab-breadcrumb.tsx app/prompt-lab/
git commit -m "feat: add collapsible PromptLab menu and breadcrumbs"
```

---

## 验收标准

1. Sidebar 中 PromptLab 默认收起，点击可展开显示 5 个子菜单项
2. 各子页面头部显示正确的面包屑：`首页 > PromptLab > [当前页面]`
3. 面包屑中 "PromptLab" 可点击打开 DropdownMenu 切换到其他子页面
4. 样式与现有 shadcn/ui 风格一致
