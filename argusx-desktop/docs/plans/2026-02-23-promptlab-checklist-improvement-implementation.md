# PromptLab Checklist 功能改进实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**目标:** 启用真实后端连接、增强列表页卡片显示、添加详情查看页面、修复 accessibility 问题

**架构:** 修改前端 API 层直接调用 Tauri 后端，新增详情页路由，增强现有卡片组件

**技术栈:** Next.js 16, Tauri v2, TypeScript, Tailwind CSS v4, shadcn/ui

---

## Task 1: 启用真实后端

**文件:**
- 修改: `lib/api/prompt-lab.ts:1-6`

**Step 1: 修改 API 调用方式**

```typescript
// 原来:
const invoke = process.env.NODE_ENV === "development" ? mockInvoke : originalInvoke;

// 改为:
const invoke = originalInvoke;
```

**Step 2: 移除 mock import**

```typescript
// 删除这一行:
import { mockInvoke } from "@/lib/mocks/prompt-lab-mock";
```

**Step 3: 测试**

运行 `pnpm tauri dev` 启动应用，确认数据从后端加载。

---

## Task 2: 添加 get_checklist_item API

**文件:**
- 修改: `src-tauri/src/lib.rs:516-528`

**Step 1: 添加 get_checklist_item 命令**

在 `list_checklist_items` 之后添加:

```rust
#[tauri::command]
async fn get_checklist_item(
    state: State<'_, Arc<PromptLab>>,
    id: i64,
) -> Result<ChecklistItemResponse, ApiError> {
    let result = state
        .checklist_service()
        .get(id)
        .await
        .map_err(ApiError::from)?;
    Ok(result.into())
}
```

**Step 2: 注册命令**

在 `invoke_handler` 中添加 `get_checklist_item`。

**Step 3: 前端添加 API 函数**

在 `lib/api/prompt-lab.ts` 添加:

```typescript
export async function getChecklistItem(id: number): Promise<ChecklistItem> {
  return invoke<ChecklistItem>("get_checklist_item", { id });
}
```

---

## Task 3: 增强列表页卡片

**文件:**
- 修改: `app/prompt-lab/checklist/page.tsx`

**Step 1: 修改卡片显示**

```tsx
// 原来:
<Card key={item.id}>
  <CardHeader className="flex flex-row items-center justify-between">
    <CardTitle>{item.name}</CardTitle>
    ...
  </CardHeader>
  <CardContent>
    <p className="text-sm text-muted-foreground line-clamp-2">{item.prompt}</p>
    <div className="flex justify-end gap-2 mt-4">
      <Button variant="outline" size="sm">
        <Pencil className="h-4 w-4" />
      </Button>
      ...
    </div>
  </CardContent>
</Card>

// 改为:
<Card key={item.id} className="cursor-pointer hover:bg-muted/50" onClick={() => router.push(`/prompt-lab/checklist/${item.id}`)}>
  <CardHeader className="flex flex-row items-center justify-between">
    <CardTitle>{item.name}</CardTitle>
    ...
  </CardHeader>
  <CardContent>
    <p className="text-sm text-muted-foreground line-clamp-2">
      {item.prompt.slice(0, 100)}{item.prompt.length > 100 ? "..." : ""}
    </p>
    <div className="flex justify-end gap-2 mt-4">
      <Button variant="outline" size="sm" onClick={(e) => { e.stopPropagation(); router.push(`/prompt-lab/checklist/${item.id}`); }}>
        <Eye className="h-4 w-4" aria-label="View" />
      </Button>
      <Button variant="outline" size="sm" onClick={(e) => { e.stopPropagation(); handleEdit(item); }}>
        <Pencil className="h-4 w-4" aria-label="Edit" />
      </Button>
      ...
    </div>
  </CardContent>
</Card>
```

**Step 2: 添加 Eye icon import**

```typescript
import { Plus, Pencil, Trash2, Check, Eye } from "lucide-react";
```

**Step 3: 添加 handleEdit 状态和函数**

```tsx
const [editingItem, setEditingItem] = useState<ChecklistItem | null>(null);

const handleEdit = (item: ChecklistItem) => {
  setEditingItem(item);
  setName(item.name);
  setPrompt(item.prompt);
  setTargetLevel(item.target_level);
  setIsCreating(true);
};
```

**Step 4: 修改创建表单支持编辑**

在表单中添加编辑模式的判断:

```tsx
<CardTitle>{editingItem ? "Edit Checklist Item" : "New Checklist Item"}</CardTitle>

// 按钮:
<Button
  onClick={editingItem ? handleUpdate : handleCreate}
  disabled={submitting || !name.trim() || !prompt.trim()}
>
  {submitting ? (editingItem ? "Updating..." : "Creating...") : (editingItem ? "Update" : "Create")}
</Button>
```

**Step 5: 添加 updateChecklistItem API 调用**

```tsx
const handleUpdate = async () => {
  if (!editingItem || !name.trim() || !prompt.trim()) return;
  setSubmitting(true);
  try {
    const updated = await updateChecklistItem({
      id: editingItem.id,
      name: name.trim(),
      prompt: prompt.trim(),
      target_level: targetLevel,
    });
    setItems(items.map(i => i.id === updated.id ? updated : i));
    setIsCreating(false);
    setEditingItem(null);
    setName("");
    setPrompt("");
    setTargetLevel("step");
  } finally {
    setSubmitting(false);
  }
};
```

---

## Task 4: 创建详情页

**文件:**
- 创建: `app/prompt-lab/checklist/[id]/page.tsx`

**Step 1: 创建详情页组件**

```tsx
"use client";

import { useState, useEffect } from "react";
import { useRouter, useParams } from "next/navigation";
import { ArrowLeft, Pencil, Calendar, Link2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { getChecklistItem, listCheckResults, type ChecklistItem } from "@/lib/api/prompt-lab";

export default function ChecklistDetailPage() {
  const router = useRouter();
  const params = useParams();
  const id = Number(params.id);

  const [item, setItem] = useState<ChecklistItem | null>(null);
  const [results, setResults] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      getChecklistItem(id),
      listCheckResults({ check_item_id: id }),
    ]).then(([itemData, resultsData]) => {
      setItem(itemData);
      setResults(resultsData);
      setLoading(false);
    });
  }, [id]);

  if (loading) return <div>Loading...</div>;
  if (!item) return <div>Not found</div>;

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="sm" onClick={() => router.back()}>
          <ArrowLeft className="h-4 w-4 mr-2" />
          Back
        </Button>
        <h1 className="text-2xl font-bold">Checklist Item</h1>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle className="text-xl">{item.name}</CardTitle>
            <div className="flex gap-2">
              <Badge variant={item.status === "active" ? "default" : "secondary"}>
                {item.status}
              </Badge>
              <Badge variant="outline">{item.target_level}</Badge>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <h3 className="text-sm font-medium mb-2">Prompt</h3>
            <pre className="p-3 bg-muted rounded-md overflow-auto max-h-60 text-sm whitespace-pre-wrap">
              {item.prompt}
            </pre>
          </div>

          {item.result_schema && (
            <div>
              <h3 className="text-sm font-medium mb-2">Result Schema</h3>
              <pre className="p-3 bg-muted rounded-md text-sm">
                {JSON.stringify(item.result_schema, null, 2)}
              </pre>
            </div>
          )}

          <div className="flex gap-4 text-sm text-muted-foreground">
            <span>Version: {item.version}</span>
          </div>

          <div className="flex gap-4 text-sm text-muted-foreground">
            <div className="flex items-center gap-1">
              <Calendar className="h-4 w-4" />
              <span>Created: {new Date(item.created_at).toLocaleString()}</span>
            </div>
            <div className="flex items-center gap-1">
              <Calendar className="h-4 w-4" />
              <span>Updated: {new Date(item.updated_at).toLocaleString()}</span>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Check History</CardTitle>
        </CardHeader>
        <CardContent>
          {results.length === 0 ? (
            <p className="text-muted-foreground">No check history yet.</p>
          ) : (
            <ul className="space-y-2">
              {results.slice(0, 5).map((result) => (
                <li
                  key={result.id}
                  className="flex items-center justify-between p-2 rounded-md bg-muted cursor-pointer hover:bg-muted/80"
                  onClick={() => router.push(`/prompt-lab/check-results/${result.id}`)}
                >
                  <div className="flex items-center gap-2">
                    <Badge variant={result.is_pass ? "default" : "destructive"}>
                      {result.is_pass ? "Pass" : "Fail"}
                    </Badge>
                    <span className="text-sm">{result.source_type}</span>
                  </div>
                  <span className="text-sm text-muted-foreground">
                    {new Date(result.created_at).toLocaleString()}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Associated Golden Sets</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground text-sm">
            Golden Set 关联功能待实现
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
```

---

## Task 5: 修复 Accessibility 问题

**文件:**
- 修改: `app/prompt-lab/checklist/page.tsx`

**Step 1: 为 icon 按钮添加 aria-label**

```tsx
// Pencil button:
<Button variant="outline" size="sm" aria-label="Edit item">
  <Pencil className="h-4 w-4" />
</Button>

// Delete button:
<Button variant="outline" size="sm" aria-label="Delete item" onClick={() => handleDelete(item.id)}>
  <Trash2 className="h-4 w-4" />
</Button>

// Eye button:
<Button variant="outline" size="sm" aria-label="View item">
  <Eye className="h-4 w-4" />
</Button>
```

**Step 2: 为 select 添加样式**

```tsx
<select
  className="w-full border rounded-md px-3 py-2 bg-background color-foreground"
  ...
>
```

---

## 验证步骤

1. 运行 `pnpm tauri dev` 启动应用
2. 导航到 `/prompt-lab/checklist`
3. 验证列表页显示真实数据
4. 点击卡片或查看按钮进入详情页
5. 验证详情页显示完整信息
6. 测试编辑功能
7. 验证 accessibility (键盘导航、aria-label)
