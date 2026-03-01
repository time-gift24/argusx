# LLM 文档标注系统前端设计

## 背景

需要为评测文本页面提供标注能力。页面结构已明确：

1. 整体左右布局：左侧评测文本，右侧固定标注栏。
2. 左侧上半为基本信息折叠（等价普通表单）。
3. 左侧下半为段落折叠，内部再次左右布局：左侧树形导航，右侧段落内容表单。
4. 段落内容表单中包含 4 个 Quill 富文本区域。

目标是让普通表单字段与富文本选区都可发起标注，并统一进入右侧单条编辑流程。

## 目标与非目标

### 目标

1. 普通字段点击可直接发起标注。
2. 富文本选区在用户停顿后再发起标注，避免立即打断。
3. 标注表单结构固定：位置信息字段（拆分）+ 违规检查项 + 动态预置字段。
4. 规则来源采用后端优先、前端兜底。
5. 文本编辑后尽可能防止标注漂移，无法确定时明确降级。

### 非目标

1. 本阶段不实现后端规则运营平台。
2. 不支持同一位置多条标注并行编辑。
3. 不实现复杂多人协同冲突解决。

## 已确认产品决策

1. 右侧使用固定栏，不使用 Modal/抽屉。
2. 右侧一次只编辑单条标注。
3. 未提交状态下切换目标：自动草稿并切换。
4. 同一位置只允许一条标注；再次操作进入编辑已有标注。
5. 自动草稿 + 显式提交。
6. 富文本触发延迟为 300ms。
7. 位置模型采用统一结构。
8. `start_offset/end_offset` 字段名保留，但语义定义为 Quill index。

## 前端架构

- `ReviewWorkspace`：页面总容器，左右布局。
- `LeftReviewPane`：评测文本区域。
- `BasicInfoAccordion`：上折叠普通表单。
- `ParagraphAccordion`：下折叠容器。
- `ParagraphTreeNav`：段落树导航。
- `ParagraphFormContent`：段落详情表单（含 4 个 Quill）。
- `RightAnnotationPanel`：固定右侧标注编辑栏。
- `AnchorCapture`：统一捕获普通字段点击与 Quill 选区事件。
- `RuleCatalogProvider`：后端规则加载与前端兜底。
- `DriftGuard`：Quill 变更后的锚点重计算与校验。

状态建议使用现有 `zustand` 体系扩展：

- `activeLocation`
- `activeAnnotationId`
- `draft`
- `annotationsByLocation`
- `ruleCatalog`
- `orphanedAnnotations`

## 统一位置模型

位置模型字段拆分如下：

1. `source_type`：`plain_field` 或 `rich_text_selection`。
2. `panel`：一级区域，如 `basic_info`、`paragraph_detail`。
3. `section_id`：折叠分组或业务分段稳定 ID。
4. `field_key`：具体字段键（表单 schema key）。
5. `node_id`：字段内锚点，富文本场景使用块级稳定 ID。
6. `start_offset`：Quill 全局 index 起始位置。
7. `end_offset`：Quill 全局 index 结束位置（建议 end-exclusive）。
8. `selected_text`：选中文本快照，用于校验与兜底重定位。

说明：

- 普通表单场景 `start_offset/end_offset` 可为空。
- Quill 的“全局 index”是单个 editor 内从开头开始的一维字符位置，换行同样计入位置。

## 交互设计

### 普通表单标注

1. 用户点击可标注字段。
2. 生成 `location`（普通字段无选区 offset）。
3. 打开右侧编辑栏。
4. 若该位置已有标注，进入“编辑已有标注”。

### 富文本标注

1. 监听 `selection-change`。
2. 当 `range.length > 0` 时启动 300ms debounce。
3. 300ms 内选区变化或清空则取消触发。
4. debounce 到时生成 `location`（含 Quill index 与 `selected_text`）并打开右栏。

### 右侧单条状态机

- `idle`：无目标。
- `editing`：编辑中。
- `draft-saving`：草稿自动保存中。
- `draft-error`：草稿保存失败可重试。
- `submit-ready`：校验通过可提交。
- `submitted`：提交成功提示态。

### 切换行为

当用户在未提交时点击新位置：

1. 触发当前编辑项自动草稿。
2. 无论草稿结果如何允许切换到新目标。
3. 若草稿失败，右侧保留可见错误并提供重试。

## 标注表单结构

右侧固定三段：

1. 位置信息（拆分字段展示，只读）。
2. 违规检查项（下拉选择）。
3. 动态预置字段（由检查项 schema 决定）。

数据对象建议：

- `AnnotationDraft`: `id`, `location`, `rule_code`, `rule_payload`, `status`, `updated_at`
- `RuleCatalogItem`: `code`, `label`, `description`, `schema`, `version`

## 规则目录策略（后端优先 + 前端兜底）

1. 页面初始化请求后端规则目录。
2. 成功则采用后端配置并缓存版本。
3. 失败则降级到前端静态规则配置。
4. 版本变化时进行草稿兼容映射（缺失字段告警但不阻塞编辑）。

## 防漂移设计

核心策略为“三层锚点 + Delta 增量重定位”：

1. 结构锚点：`panel + section_id + field_key + node_id`
2. 位置锚点：`start_offset/end_offset`（Quill index）
3. 文本锚点：`selected_text`

处理流程：

1. 监听 Quill `text-change(delta)`。
2. 仅对同 editor 关联标注进行位置变换（基于 Delta 位置转换）。
3. 重算后回读当前文本片段，与 `selected_text` 校验。
4. 校验失败时在近邻窗口搜索文本快照：
   - 唯一命中：自动重挂并记录 `reanchored_auto`。
   - 多命中或无命中：标记 `orphaned`。
5. `orphaned` 标注需要用户手动确认重挂，不可直接提交最终态。

设计原则：宁可降级为 `orphaned`，也不静默挂到错误文本。

## 错误处理与可用性

1. 规则加载失败：降级并提示“当前使用本地规则集”。
2. 草稿保存失败：不阻断编辑与切换，保留失败状态可重试。
3. 提交失败：保持草稿态并给出错误信息。
4. Quill 未就绪：不注册选区触发，待 ready 后再启用。
5. 左侧高亮：普通字段与富文本统一表现“可编辑已有标注”。

## 性能策略

1. 富文本触发使用 300ms debounce。
2. 自动草稿保存使用节流（建议 800ms）。
3. 变更重算仅限当前 editor 下关联标注。
4. 批量重算使用微任务/分帧，避免输入卡顿。

## 测试与验收

### 单元测试

1. `location` 生成正确。
2. 同位置唯一判定正确。
3. 规则驱动动态表单渲染与校验正确。
4. 自动草稿状态机流转正确。
5. Delta 重算与 `orphaned` 降级逻辑正确。

### 集成测试

1. 普通字段点击可发起标注。
2. 富文本选区在 300ms 后触发。
3. 未提交切换时自动草稿并切换。
4. 提交后左侧高亮状态同步更新。
5. 后端规则不可用时可无阻塞降级。

### 验收标准

1. 普通字段和 4 个 Quill 都能稳定发起标注。
2. 右侧始终单条编辑且不会并发冲突。
3. 用户输入在切换时不会丢失（至少保留草稿）。
4. 文本变化后不出现静默错位，无法确认时明确 `orphaned`。
5. 规则目录在后端/前端双源下可持续可用。
