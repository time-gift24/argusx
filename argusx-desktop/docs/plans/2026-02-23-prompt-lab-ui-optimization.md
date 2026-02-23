# PromptLab UI 优化实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 全面优化 PromptLab 界面，实现九宫格 Dashboard、卡片信息增强、模块关联展示

**Architecture:** 采用组件化设计，提取通用 Card 组件，在 Dashboard 和列表页面复用

**Tech Stack:** Next.js 16, React, Tailwind CSS v4, shadcn/ui

---

## 准备工作

### Task 0: 创建开发分支

**Step 1: 创建并切换到新分支**

```bash
git checkout -b feature/prompt-lab-ui-optimization
```

**Step 2: 验证当前目录**

```bash
pwd
# Expected: /Users/wanyaozhong/Projects/argusx-b/argusx-desktop
```

---

## Task 1: 优化 Dashboard 页面

**Files:**
- Modify: `app/prompt-lab/page.tsx`

**Step 1: 读取当前 Dashboard 代码**

```bash
cat app/prompt-lab/page.tsx
```

**Step 2: 重写 Dashboard 为九宫格布局**

```tsx
"use client";

import { useState, useEffect } from "react";
import { CheckCircle, XCircle, Folder, FileText, ArrowRight } from "lucide-react";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
  listChecklistItems,
  listGoldenSetItems,
  listCheckResults,
  listAiExecutionLogs,
} from "@/lib/api/prompt-lab";

interface ModuleStats {
  checklist: number;
  goldenSets: number;
  results: { passed: number; failed: number };
  logs: number;
}

const modules = [
  { key: "checklist", name: "Checklist", icon: CheckCircle, href: "/prompt-lab/checklist" },
  { key: "goldenSets", name: "Golden Sets", icon: Folder, href: "/prompt-lab/golden-sets" },
  { key: "results", name: "Results", icon: FileText, href: "/prompt-lab/results" },
  { key: "logs", name: "Logs", icon: FileText, href: "/prompt-lab/logs" },
];

export default function PromptLabDashboard() {
  const [stats, setStats] = useState<ModuleStats>({
    checklist: 0,
    goldenSets: 0,
    results: { passed: 0, failed: 0 },
    logs: 0,
  });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      listChecklistItems({}),
      listGoldenSetItems(1),
      listCheckResults({}),
      listAiExecutionLogs({}),
    ]).then(([items, goldenItems, results, logs]) => {
      setStats({
        checklist: items.length,
        goldenSets: goldenItems.length,
        results: {
          passed: results.filter((r) => r.is_pass).length,
          failed: results.filter((r) => !r.is_pass).length,
        },
        logs: logs.length,
      });
      setLoading(false);
    });
  }, []);

  const getModuleValue = (key: string) => {
    switch (key) {
      case "checklist": return stats.checklist;
      case "goldenSets": return stats.goldenSets;
      case "results": return stats.results.passed + stats.results.failed;
      case "logs": return stats.logs;
      default: return 0;
    }
  };

  const getModuleSubtitle = (key: string) => {
    switch (key) {
      case "checklist": return "items";
      case "goldenSets": return "sets";
      case "results": return `${stats.results.passed} passed, ${stats.results.failed} failed`;
      case "logs": return "entries";
      default: return "";
    }
  };

  if (loading) {
    return <div className="p-8 text-center text-muted-foreground">Loading...</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">PromptLab</h1>
      </div>

      {/* 九宫格模块卡片 */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {modules.map((module) => (
          <Card key={module.key} className="hover:shadow-md transition-shadow cursor-pointer group">
            <CardHeader className="flex flex-row items-center gap-3 pb-2">
              <div className="p-2 rounded-lg bg-primary/10">
                <module.icon className="h-5 w-5 text-primary" />
              </div>
              <CardTitle className="text-base font-semibold">{module.name}</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                <div className="text-3xl font-bold tabular-nums">{getModuleValue(module.key)}</div>
                <div className="text-sm text-muted-foreground">{getModuleSubtitle(module.key)}</div>
                <Button variant="ghost" size="sm" className="w-full opacity-0 group-hover:opacity-100 transition-opacity">
                  查看详情 <ArrowRight className="h-4 w-4 ml-1" />
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* 数据流向可视化 */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">数据流向</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-center gap-2 md:gap-4 text-sm">
            <div className="flex items-center gap-2">
              <span className="px-3 py-1.5 rounded-md bg-primary/10 text-primary font-medium">
                Checklist ({stats.checklist})
              </span>
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
            </div>
            <div className="flex items-center gap-2">
              <span className="px-3 py-1.5 rounded-md bg-primary/10 text-primary font-medium">
                Golden Sets ({stats.goldenSets})
              </span>
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
            </div>
            <div className="flex items-center gap-2">
              <span className="px-3 py-1.5 rounded-md bg-green-500/10 text-green-600 font-medium">
                Passed ({stats.results.passed})
              </span>
              <span className="px-3 py-1.5 rounded-md bg-red-500/10 text-red-600 font-medium">
                Failed ({stats.results.failed})
              </span>
            </div>
            <div className="flex items-center gap-2">
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
              <span className="px-3 py-1.5 rounded-md bg-muted text-muted-foreground font-medium">
                Logs ({stats.logs})
              </span>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
```

**Step 3: 提交更改**

```bash
git add app/prompt-lab/page.tsx
git commit -m "feat(prompt-lab): optimize dashboard with grid layout and data flow"
```

---

## Task 2: 优化 Checklist 列表页面

**Files:**
- Modify: `app/prompt-lab/checklist/page.tsx`

**Step 1: 读取当前 Checklist 代码**

```bash
cat app/prompt-lab/checklist/page.tsx
```

**Step 2: 增强卡片显示关联信息和时间**

在 CardContent 中添加：
- 关联 Golden Set 信息（从 listGoldenSetItems 获取）
- 检查结果统计
- 创建和更新时间

```tsx
// 在 items.map 中添加关联信息
{items.map((item) => (
  <Card key={item.id} className="cursor-pointer hover:bg-muted/50 transition-colors">
    <CardHeader className="flex flex-row items-center justify-between">
      <CardTitle className="flex items-center gap-2">
        <CheckCircle className="h-5 w-5 text-green-500" />
        {item.name}
      </CardTitle>
      <div className="flex items-center gap-2">
        <Badge variant={item.status === "active" ? "default" : "secondary"}>
          {item.status}
        </Badge>
        <Badge variant="outline">{item.target_level}</Badge>
      </div>
    </CardHeader>
    <CardContent>
      <p className="text-sm text-muted-foreground line-clamp-2 mb-3">
        {item.prompt.slice(0, 100)}
        {item.prompt.length > 100 ? "..." : ""}
      </p>

      {/* 新增：关联信息和时间 */}
      <div className="flex flex-wrap gap-4 text-xs text-muted-foreground border-t pt-3 mt-3">
        <div className="flex items-center gap-1">
          <Folder className="h-3 w-3" />
          <span>Golden Set #1</span>
        </div>
        <div className="flex items-center gap-1">
          <CheckCircle className="h-3 w-3 text-green-500" />
          <span>5 passed</span>
        </div>
        <div className="flex items-center gap-1">
          <XCircle className="h-3 w-3 text-red-500" />
          <span>1 failed</span>
        </div>
        <div className="ml-auto">
          创建: {new Date(item.created_at).toLocaleDateString()}
        </div>
      </div>

      <div className="flex justify-end gap-2 mt-4">
        {/* 现有按钮保持不变 */}
      </div>
    </CardContent>
  </Card>
))}
```

**Step 3: 提交更改**

```bash
git add app/prompt-lab/checklist/page.tsx
git commit -m "feat(prompt-lab): enhance checklist cards with关联 info and timestamps"
```

---

## Task 3: 优化 Results 列表页面

**Files:**
- Modify: `app/prompt-lab/results/page.tsx`

**Step 1: 读取当前 Results 代码**

```bash
cat app/prompt-lab/results/page.tsx
```

**Step 2: 增强 Results 卡片显示检查项详情**

```tsx
// 在 CardContent 中添加更详细的信息
<CardContent>
  <div className="space-y-3">
    {/* 检查项名称 - 关联到 Checklist */}
    <div className="flex items-center gap-2 text-sm">
      <span className="text-muted-foreground">检查项:</span>
      <span className="font-medium">Checklist Item #{result.check_item_id}</span>
    </div>

    {/* AI 执行详情 */}
    <div className="flex flex-wrap gap-4 text-sm text-muted-foreground">
      <div className="flex items-center gap-1">
        <Badge variant="outline">{result.source_type}</Badge>
      </div>
      <div>耗时: {result.latency_ms}ms</div>
      <div>Tokens: {result.input_tokens}/{result.output_tokens}</div>
    </div>

    {/* 时间 */}
    <div className="text-xs text-muted-foreground">
      {new Date(result.created_at).toLocaleString()}
    </div>
  </div>
</CardContent>
```

**Step 3: 提交更改**

```bash
git add app/prompt-lab/results/page.tsx
git commit -m "feat(prompt-lab): enhance results cards with AI execution details"
```

---

## Task 4: 优化 Golden Sets 页面

**Files:**
- Modify: `app/prompt-lab/golden-sets/page.tsx`

**Step 1: 读取当前 Golden Sets 代码**

```bash
cat app/prompt-lab/golden-sets/page.tsx
```

**Step 2: 增强显示**

添加关联 Checklist 的链接显示。

**Step 3: 提交更改**

```bash
git add app/prompt-lab/golden-sets/page.tsx
git commit -m "feat(prompt-lab): enhance golden sets cards with links"
```

---

## Task 5: 检查并优化 Logs 页面

**Files:**
- Modify: `app/prompt-lab/logs/page.tsx`

**Step 1: 检查 Logs 页面是否存在**

```bash
ls -la app/prompt-lab/logs/
```

**Step 2: 如存在则优化**

```bash
cat app/prompt-lab/logs/page.tsx
```

---

## 验收检查

**Step 1: 启动开发服务器**

```bash
pnpm dev
```

**Step 2: 访问页面**

打开浏览器访问 http://localhost:3000/prompt-lab

**Step 3: 验证清单**

- [ ] Dashboard 显示 4 个模块卡片，九宫格布局
- [ ] Dashboard 底部显示数据流向
- [ ] Checklist 卡片显示关联信息
- [ ] Results 卡片显示 AI 执行详情
- [ ] 亮色/暗色模式正常显示

**Step 4: 提交最终更改**

```bash
git add -A
git commit -m "feat(prompt-lab): complete UI optimization"
```

---

## Plan complete

**保存路径:** `docs/plans/2026-02-23-prompt-lab-ui-optimization.md`

**Two execution options:**

1. **Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

2. **Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
