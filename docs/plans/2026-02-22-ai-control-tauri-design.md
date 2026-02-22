# AI 控制 Tauri 桌面应用 - 设计文档

**日期**: 2026-02-22
**状态**: 待实现

## 1. 需求概述

让 Chat 页面中的 AI 能够通过 API 控制桌面应用（导航、CRUD、读取状态），采用 **dry-run 预览** + **用户确认** 的交互模式。

## 2. 核心流程

```
用户说"创建 checklist"
        ↓
AI 解析意图 → 返回 action + params
        ↓
Dry-run 执行 → 调用 Tauri 命令 + dry_run=true
        ↓
预览结果 → 展示"将创建：xxx"
        ↓
用户决策：
  - 确认执行 → dry_run=false → 真正执行
  - 取消 → 不执行
        ↓
执行/撤销结果 → 返回给 Chat
```

## 3. 技术实现

### 3.1 Tauri 命令增加 dry_run 参数

为所有写操作（create、update、delete）增加 `dry_run` 参数：

```rust
#[tauri::command]
async fn create_checklist_item(
    state: State<'_, Arc<PromptLab>>,
    input: CreateChecklistItemInput,
    dry_run: bool,
) -> Result<CommandPreview<ChecklistItemResponse>, ApiError> {
    if dry_run {
        // 模拟创建，返回预览
        let preview_item = ChecklistItemResponse {
            id: 0,  // 0 表示新建
            name: input.name.clone(),
            prompt: input.prompt.clone(),
            target_level: input.target_level.clone().into(),
            result_schema: input.result_schema.clone(),
            version: 1,
            status: input.status.clone().into(),
            created_at: "".to_string(),
            updated_at: "".to_string(),
            created_by: None,
            updated_by: None,
            deleted_at: None,
        };
        Ok(CommandPreview {
            action: "create_checklist_item",
            preview: Some(preview_item),
            warning: None,
            can_execute: true,
        })
    } else {
        // 真正执行
        let result = state.checklist_service()
            .create(input.into())
            .await
            .map_err(ApiError::from)?;
        Ok(CommandPreview {
            action: "create_checklist_item",
            preview: Some(result.into()),
            warning: None,
            can_execute: true,
        })
    }
}
```

### 3.2 预览响应格式

```rust
#[derive(Debug, Serialize)]
pub struct CommandPreview<T> {
    pub action: String,
    pub preview: Option<T>,
    pub warning: Option<String>,
    pub can_execute: bool,
}
```

### 3.3 支持的操作

| 操作类型 | 命令 | 说明 |
|----------|------|------|
| **create** | `create_checklist_item` | 创建 checklist |
| **update** | `update_checklist_item` | 更新 checklist |
| **delete** | `delete_checklist_item` | 删除 checklist |
| **read** | `list_checklist_items` | 获取列表（无需 dry-run） |
| **bind** | `bind_golden_set_item` | 绑定 golden set |
| **unbind** | `unbind_golden_set_item` | 解绑 golden set |
| **upsert** | `upsert_check_result` | 创建/更新检查结果 |

### 3.4 前端确认对话框设计

```
┌─────────────────────────────────────┐
│  🤖 AI 建议操作                      │
├─────────────────────────────────────┤
│  将创建 Checklist:                   │
│  • name: "测试 checklist"           │
│  • status: "active"                 │
│  • target_level: "step"             │
├─────────────────────────────────────┤
│  [取消]              [执行]          │
└─────────────────────────────────────┘
```

## 4. 待实现命令列表

需要为以下命令增加 `dry_run` 参数：

- [ ] `create_checklist_item`
- [ ] `update_checklist_item`
- [ ] `delete_checklist_item`
- [ ] `bind_golden_set_item`
- [ ] `unbind_golden_set_item`
- [ ] `upsert_check_result`

## 5. 安全考虑

- 写操作都需要用户确认
- AI 无法直接执行，必须通过用户交互
- 预览模式下不写入数据库

## 6. 后续扩展

- 导航操作（`navigate_to_page`）
- 批量操作预览
- 操作历史记录
