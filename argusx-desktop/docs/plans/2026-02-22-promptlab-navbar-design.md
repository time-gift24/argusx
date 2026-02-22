# PromptLab 导航设计方案

## 背景

当前 prompt-lab 各页面没有统一的导航入口，需要在现有 AppSidebar 基础上添加 collapsible 子菜单，并在页面头部添加面包屑导航。

## 设计方案

### 1. Sidebar 修改

将 PromptLab 从简单的菜单项改为 Collapsible + DropdownMenu 组合：

```
[Dashboard]
[PromptLab ▼]  ← Collapsible，点击展开
  ├ Checklist
  ├ Golden Sets
  ├ Results
  └ Logs
```

- 默认收起状态
- PromptLab 本身可点击跳转到 `/prompt-lab`
- 展开后显示 4 个子页面，全部可点击跳转

**实现方式：**
- 使用已有的 `Collapsible` 组件 (`components/ui/collapsible.tsx`)
- 使用 `DropdownMenu` 组件
- 参考 shadcn/ui 的 sidebar group 折叠模式

### 2. 右侧页面头部 - 面包屑

在每个 PromptLab 子页面头部添加面包屑：

```
首页 > PromptLab > [当前页面]
```

- "首页" → `/`
- "PromptLab" 位置使用 DropdownMenu，点击可快速切换到其他 PromptLab 子页面
- "[当前页面]" 显示当前页面名称，无下拉

**示例：**
- `/prompt-lab/checklist` → `首页 > PromptLab > Checklist`
- `/prompt-lab/golden-sets` → `首页 > PromptLab > Golden Sets`
- `/prompt-lab/logs` → `首页 > PromptLab > Logs`

### 3. 组件结构

需要创建/修改的组件：

1. **修改 `components/layouts/sidebar/app-sidebar.tsx`**
   - 添加 Collapsible + DropdownMenu 实现子菜单

2. **创建 `components/layouts/prompt-lab-breadcrumb.tsx`**
   - 面包屑组件，支持 DropdownMenu 切换

3. **修改各子页面**
   - `app/prompt-lab/page.tsx` (Dashboard)
   - `app/prompt-lab/checklist/page.tsx`
   - `app/prompt-lab/golden-sets/page.tsx`
   - `app/prompt-lab/results/page.tsx`
   - `app/prompt-lab/logs/page.tsx`

   每个页面顶部添加面包屑组件

## 验收标准

1. Sidebar 中 PromptLab 默认收起，点击可展开显示 4 个子页面
2. 各子页面头部显示正确的面包屑
3. 面包屑中 "PromptLab" 可点击切换到其他子页面
4. 样式与现有 shadcn/ui 风格一致
