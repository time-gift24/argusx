# SOP 标注前端结构改造设计

## 背景

当前标注页左侧“段落明细”仍是固定 4 个 Quill 区块（段落摘要/法条依据/程序描述/处理结论）。

需求已演进为 SOP 结构：

1. 上下总体结构保持不变（基础信息 + 段落明细）。
2. 基础信息需要保留现有字段，并新增 SOP 基本信息 `sop_id`、`name`。
3. 段落明细改为左右结构：左侧树形导航，右侧为当前选中 step 的 4 个固定折叠区（Quill HTML 只读）。
4. 标注定位和高亮仍需与右侧“定位信息”一致。

## 目标

1. 左侧段落明细支持 SOP 树形导航，固定 4 组：`01操作检测`、`02操作处理`、`03操作验证`、`04操作回退`。
2. 右侧详情只展示当前选中 `sop_step` 的 4 个固定字段：`operation`、`verification`、`impact_analysis`、`rollback`。
3. 每个字段使用只读 Quill 渲染 HTML，继续支持文本选区标注。
4. 定位字段编码标准化为 `field_key = sop.<sop_step_id>.<section>`。
5. 保持现有右侧标注面板、提交流程和文本高亮策略不回归。

## 非目标

1. 不做 SOP 文本编辑能力（保持只读标注）。
2. 不做后端协议重构（仅前端映射适配）。
3. 不做多 step 并排详情展示。

## 已确认决策

1. 选择某个树节点后，右侧仅展示该 `sop_step` 的内容。
2. `sop_id`、`name` 放在基础信息区域显示。
3. 页面加载后默认展开 4 组并默认选中第一条可用 `sop_step`。
4. 标注 `field_key` 使用 `sop.<sop_step_id>.<section>`。
5. 右侧 4 区块 Quill 为只读。
6. 若某区块内容缺失，仍渲染该区块并显示空占位。

## 前端架构与组件边界

### 1) 基础信息区域

在现有 `BasicInfoForm` 基础上扩展：

- 保留既有基础字段展示与点击标注能力。
- 新增 `sop_id`、`name` 展示。
- 支持透传“其他基础信息字段”，避免 schema 补齐时反复改 UI 结构。

### 2) 段落明细区域

将当前 `ParagraphPanel` 改造为 SOP 视图容器（建议拆为两个子组件）：

- `SopTreeNav`（左）
  - 固定 4 个分组节点（detect/handle/verification/rollback）。
  - 每组展示 step 列表（`name`）。
  - 分组支持展开/收起，点击 step 切换激活项。
- `SopStepDetail`（右）
  - 仅渲染当前激活 step。
  - 固定 4 个可折叠 Quill 区块：`operation`、`verification`、`impact_analysis`、`rollback`。

### 3) 右侧标注面板

`RightAnnotationPanel` 继续沿用当前逻辑，不改 store action 和提交流程。

## 数据模型与映射

## SOP 文档模型（前端视图）

```ts
type SopStepLite = {
  sop_step_id: number;
  name: string;
  version: number;
};

type SopStepDetail = {
  id: number;
  name: string;
  operation: string;
  verification: string;
  impact_analysis: string;
  rollback: string;
};

type SopDocument = {
  sop_id: string;
  name: string;
  basic_info: Record<string, string>;
  detect: SopStepLite[];
  handle: SopStepLite[];
  verification: SopStepLite[];
  rollback: SopStepLite[];
  step_details: Record<number, SopStepDetail>;
};
```

## 左树映射

- `01操作检测` <- `detect`
- `02操作处理` <- `handle`
- `03操作验证` <- `verification`
- `04操作回退` <- `rollback`

## 定位编码

对于 step 内 4 区块，统一编码：

- `section_id = sop-step-<sop_step_id>`
- `field_key = sop.<sop_step_id>.<section>`
- `node_id = <field_key>-node`

其中 `<section>` ∈ `{ operation, verification, impact_analysis, rollback }`。

## 交互与状态流

1. 页面加载后默认展开 4 组。
2. 自动选中第一条可用 step（扫描顺序：detect -> handle -> verification -> rollback）。
3. 点击分组标题仅切换分组展开态，不改变当前选中 step。
4. 点击 step 后，右侧详情切换到该 step。
5. 右侧详情固定 4 折叠区，建议默认展开 `operation`，其余默认收起。
6. Quill 选区触发规则保持：`selection-change` 停顿 300ms 后 `OPEN`。
7. 文本高亮策略保持：
   - `submitted` 持久高亮
   - 当前 active `draft` 深色高亮
   - `orphaned` 不高亮

## 缺失与异常处理

1. 某分组无 step：显示“暂无步骤”。
2. 当前 step 某字段为空：区块保留并显示空占位。
3. 所有分组都为空：右侧显示空态，不创建 Quill 实例。
4. step 详情缺失：回退为空内容，保证界面可用。

## 测试策略

## 组件测试

1. 左树默认展开 + 默认激活第一条 step。
2. 点击 step 后右侧详情切换。
3. 右侧固定 4 区块始终渲染。
4. 字段缺失时显示占位而非隐藏区块。

## 标注回归

1. 选区 300ms 触发 OPEN。
2. location 生成正确：`section_id/field_key/start_offset/end_offset/selected_text`。
3. 文本高亮规则不回归（submitted/draft/orphaned）。
4. 右侧提交按钮状态链路不回归（禁用、提交中、已提交）。

## 验收标准

1. 基础信息区保留既有字段并新增 `sop_id/name`。
2. 段落明细呈现 SOP 树 + 单 step 详情双栏结构。
3. 右侧仅展示当前 step 的 4 个 Quill 折叠区。
4. 定位信息和文本高亮与用户选区一致。
5. 现有右侧标注提交流程可用且无回归。
