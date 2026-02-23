# PromptLab Checklist 功能改进设计

## 概述

改进 PromptLab 的 Checklist 页面：启用真实后端连接、增强卡片信息、添加详情查看页面。

## 目标

1. 移除前端 mock，直接使用 Tauri 后端
2. 增强列表页卡片显示信息
3. 添加 Checklist Item 详情页
4. 修复 accessibility 问题

---

## 1. 启用真实后端

### 现状
- `lib/api/prompt-lab.ts` 使用 `process.env.NODE_ENV === "development"` 判断
- Next.js 开发模式 `pnpm dev` 时 `NODE_ENV` 是 `development`，导致一直使用 mock

### 方案
- 移除 mock 判断，直接使用 `invoke` 调用 Tauri 后端
- 开发时需要启动 `pnpm tauri dev` 运行完整 Tauri 应用

### 修改文件
- `lib/api/prompt-lab.ts`

---

## 2. 列表页卡片增强

### 现状
- 只显示: name, status badge, target_level badge, prompt (line-clamp-2)
- 编辑按钮是空壳，无功能

### 改进后
- 显示: name, status badge, target_level badge
- prompt 预览 (前 100 字符 + "...")
- 添加 "查看" 按钮
- 添加 "编辑" 功能 (实现编辑功能)

### 修改文件
- `app/prompt-lab/checklist/page.tsx`

---

## 3. 详情页设计

### 路由
`/prompt-lab/checklist/[id]`

### 页面结构
```
┌─────────────────────────────────────────┐
│ ← 返回  |  Edit                         │
├─────────────────────────────────────────┤
│ Name                          Status   │
├─────────────────────────────────────────┤
│ Target Level: step          Version: 1│
├─────────────────────────────────────────┤
│ Prompt                                    │
│ ┌─────────────────────────────────────┐  │
│ │ (完整 prompt 内容，支持滚动)         │  │
│ └─────────────────────────────────────┘  │
├─────────────────────────────────────────┤
│ Result Schema (如果有)                   │
│ ┌─────────────────────────────────────┐  │
│ │ { "type": "boolean" }              │  │
│ └─────────────────────────────────────┘  │
├─────────────────────────────────────────┤
│ 时间信息                                 │
│ 创建: 2026-02-22 10:30                  │
│ 更新: 2026-02-22 14:20                  │
├─────────────────────────────────────────┤
│ 关联的 Golden Sets (可点击跳转)         │
│ • Set A (绑定于 2026-02-20) →           │
│ • Set B (绑定于 2026-02-21) →           │
├─────────────────────────────────────────┤
│ 检查历史 (可点击跳转)                    │
│ • 2026-02-22 AI检查 - 通过              │
│ • 2026-02-21 手动检查 - 未通过          │
└─────────────────────────────────────────┘
```

### 交互
- 点击 "返回" 回到列表页
- 点击 "Edit" 进入编辑模式 (inline 或 modal)
- 点击 Golden Set 跳转至 `/prompt-lab/golden-sets/[id]`
- 点击检查历史跳转至对应详情

---

## 4. 需补充的 API

### 后端 (Rust)
- `get_checklist_item(id)` - 获取单个详情
- `list_golden_sets()` - 列出所有 Golden Sets
- `list_check_results_for_item(checklist_item_id)` - 获取某 item 的检查历史

### 前端
- 添加对应的 TypeScript 类型和 API 函数

---

## 5. Accessibility 修复

- Icon 按钮添加 `aria-label`
- 原生 select 添加背景色和颜色样式 (Windows dark mode)
- 卡片添加键盘处理 (可选)

---

## 修改文件清单

### 前端
1. `lib/api/prompt-lab.ts` - 移除 mock，直接用 invoke
2. `app/prompt-lab/checklist/page.tsx` - 增强卡片，添加查看/编辑按钮
3. `app/prompt-lab/checklist/[id]/page.tsx` - 新增详情页

### 后端 (Rust)
4. `src-tauri/src/lib.rs` - 新增 `get_checklist_item` 命令

---

## 优先级

1. 启用真实后端 (阻断性)
2. 列表页卡片增强
3. 详情页实现
4. API 补充
5. Accessibility 修复
