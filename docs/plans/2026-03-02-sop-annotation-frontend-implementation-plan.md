# SOP 标注前端改造 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将“段落明细”从固定段落字段改造为 SOP 树 + 单 step 四区块详情，同时保持现有标注、提交流程与高亮策略不回归。

**Architecture:** 在现有 `AnnotationWorkspace` 总体布局不变的前提下，将 `ParagraphPanel` 拆分为左侧 `SopTreeNav` 和右侧 `SopStepDetail`。通过新增 SOP 视图模型工具函数统一处理默认选中、字段映射和 `field_key` 编码，复用现有 `QuillReviewField` 的选区标注与高亮机制。右侧 `RightAnnotationPanel`、store/reducer/action 不做协议变更。

**Tech Stack:** Next.js 16 + React + TypeScript + Zustand + Quill + Vitest + Testing Library + ESLint。

---

## Execution Rules

- 严格按任务顺序执行，不跳步。
- 每个任务必须遵循 `fail -> pass -> commit`。
- 每个任务完成后记录：改动文件、测试命令、结果、commit hash。
- 开发过程遵循 @test-driven-development 与 @verification-before-completion。

---

### Task 1: SOP 视图模型与字段编码工具

**Files:**
- Create: `desktop/lib/annotation/sop-view-model.ts`
- Create: `desktop/lib/annotation/sop-view-model.test.ts`

**Step 1: Write the failing test**

在 `sop-view-model.test.ts` 新增用例：

```ts
import { describe, expect, it } from "vitest";
import {
  buildSopFieldKey,
  buildSopSectionId,
  pickDefaultSopStep,
} from "@/lib/annotation/sop-view-model";

describe("sop view model", () => {
  it("builds field_key and section_id with sop_step_id", () => {
    expect(buildSopFieldKey(12, "operation")).toBe("sop.12.operation");
    expect(buildSopSectionId(12)).toBe("sop-step-12");
  });

  it("picks first available step in detect->handle->verification->rollback order", () => {
    const step = pickDefaultSopStep({
      detect: [],
      handle: [{ sop_step_id: 21, name: "处理A", version: 1 }],
      verification: [{ sop_step_id: 31, name: "验证A", version: 1 }],
      rollback: [],
    });

    expect(step?.sop_step_id).toBe(21);
  });
});
```

**Step 2: Run test to verify it fails**

Run:

```bash
cd desktop && pnpm test -- lib/annotation/sop-view-model.test.ts
```

Expected: FAIL，报错 `Cannot find module '@/lib/annotation/sop-view-model'`。

**Step 3: Write minimal implementation**

在 `sop-view-model.ts` 实现：

```ts
export type SopCategory = "detect" | "handle" | "verification" | "rollback";
export type SopSection = "operation" | "verification" | "impact_analysis" | "rollback";

export type SopStepLite = {
  sop_step_id: number;
  name: string;
  version: number;
};

export type SopGroups = {
  detect: SopStepLite[];
  handle: SopStepLite[];
  verification: SopStepLite[];
  rollback: SopStepLite[];
};

export function buildSopFieldKey(stepId: number, section: SopSection): string {
  return `sop.${stepId}.${section}`;
}

export function buildSopSectionId(stepId: number): string {
  return `sop-step-${stepId}`;
}

export function pickDefaultSopStep(groups: SopGroups): SopStepLite | null {
  const order: SopCategory[] = ["detect", "handle", "verification", "rollback"];
  for (const key of order) {
    if (groups[key].length > 0) {
      return groups[key][0];
    }
  }
  return null;
}
```

**Step 4: Run test to verify it passes**

Run:

```bash
cd desktop && pnpm test -- lib/annotation/sop-view-model.test.ts
```

Expected: PASS。

**Step 5: Commit**

```bash
git add desktop/lib/annotation/sop-view-model.ts desktop/lib/annotation/sop-view-model.test.ts
git commit -m "feat(annotation): add sop view-model helpers"
```

---

### Task 2: SOP 树组件（左侧导航）

**Files:**
- Create: `desktop/components/features/annotation/sop-tree-nav.tsx`
- Create: `desktop/components/features/annotation/sop-tree-nav.test.tsx`

**Step 1: Write the failing test**

在测试中覆盖：
- 固定 4 组标题显示。
- 默认全部展开。
- 点击某个 step 会触发 `onSelect(stepId)`。

示例断言：

```ts
expect(screen.getByText("01操作检测")).toBeInTheDocument();
expect(screen.getByRole("button", { name: "处理A" })).toBeInTheDocument();
await user.click(screen.getByRole("button", { name: "处理A" }));
expect(onSelect).toHaveBeenCalledWith(21);
```

**Step 2: Run test to verify it fails**

Run:

```bash
cd desktop && pnpm test -- components/features/annotation/sop-tree-nav.test.tsx
```

Expected: FAIL，组件文件不存在。

**Step 3: Write minimal implementation**

实现 `SopTreeNav`：
- props：`groups`, `activeStepId`, `onSelect`。
- 4 组固定 `details open`。
- 每条 step 渲染 `button`，激活态有 class。

**Step 4: Run test to verify it passes**

Run:

```bash
cd desktop && pnpm test -- components/features/annotation/sop-tree-nav.test.tsx
```

Expected: PASS。

**Step 5: Commit**

```bash
git add desktop/components/features/annotation/sop-tree-nav.tsx desktop/components/features/annotation/sop-tree-nav.test.tsx
git commit -m "feat(annotation): add sop tree navigation"
```

---

### Task 3: SOP 单 step 详情组件（右侧四区块）

**Files:**
- Create: `desktop/components/features/annotation/sop-step-detail.tsx`
- Create: `desktop/components/features/annotation/sop-step-detail.test.tsx`

**Step 1: Write the failing test**

测试覆盖：
- 固定渲染 4 个折叠区。
- 每个区块都向 `QuillReviewField` 传入 `fieldKey = sop.<id>.<section>`。
- 字段缺失时仍渲染区块并传空文本。

建议在测试里 `vi.mock("./quill-review-field")`，只断言 props。

**Step 2: Run test to verify it fails**

Run:

```bash
cd desktop && pnpm test -- components/features/annotation/sop-step-detail.test.tsx
```

Expected: FAIL，组件不存在。

**Step 3: Write minimal implementation**

`SopStepDetail` 实现要点：
- props：`stepId`, `stepName`, `stepDetail`。
- 固定 sections：`operation/verification/impact_analysis/rollback`。
- section 渲染顺序固定。
- `sectionId` 使用 `buildSopSectionId(stepId)`。
- `fieldKey` 使用 `buildSopFieldKey(stepId, section)`。
- 缺值使用 `""`。

**Step 4: Run test to verify it passes**

Run:

```bash
cd desktop && pnpm test -- components/features/annotation/sop-step-detail.test.tsx
```

Expected: PASS。

**Step 5: Commit**

```bash
git add desktop/components/features/annotation/sop-step-detail.tsx desktop/components/features/annotation/sop-step-detail.test.tsx
git commit -m "feat(annotation): add sop step detail with quill sections"
```

---

### Task 4: 改造 ParagraphPanel 为 SOP 双栏联动

**Files:**
- Modify: `desktop/components/features/annotation/paragraph-panel.tsx`
- Modify: `desktop/components/features/annotation/mock-review-data.ts`
- Create: `desktop/components/features/annotation/paragraph-panel.test.tsx`

**Step 1: Write the failing test**

新增 `paragraph-panel.test.tsx` 覆盖：
- 渲染后存在左树 4 组。
- 默认显示第一条 step 对应详情。
- 点击左树其他 step 后，右侧标题切换到目标 step。

**Step 2: Run test to verify it fails**

Run:

```bash
cd desktop && pnpm test -- components/features/annotation/paragraph-panel.test.tsx
```

Expected: FAIL（当前 `ParagraphPanel` 仍是旧结构）。

**Step 3: Write minimal implementation**

改造 `paragraph-panel.tsx`：
- 读取 `mockReviewData.sop`（含 groups + step_details）。
- 使用 `pickDefaultSopStep` 初始化 `activeStepId`。
- 左侧渲染 `SopTreeNav`。
- 右侧渲染 `SopStepDetail`。
- 空态分支（无 step）显示占位。

同步扩展 `mock-review-data.ts`，补充 SOP 示例数据（至少每组 1 条，便于联动测试）。

**Step 4: Run test to verify it passes**

Run:

```bash
cd desktop && pnpm test -- components/features/annotation/paragraph-panel.test.tsx
```

Expected: PASS。

**Step 5: Commit**

```bash
git add desktop/components/features/annotation/paragraph-panel.tsx desktop/components/features/annotation/mock-review-data.ts desktop/components/features/annotation/paragraph-panel.test.tsx
git commit -m "feat(annotation): switch paragraph panel to sop tree layout"
```

---

### Task 5: 基础信息区保留并扩展 sop_id/name

**Files:**
- Modify: `desktop/components/features/annotation/basic-info-form.tsx`
- Create: `desktop/components/features/annotation/basic-info-form.test.tsx`

**Step 1: Write the failing test**

新增用例验证：
- 保留原 `case_title/case_summary`。
- 新增展示 `sop_id/name`。
- 点击 `sop_id/name` 也可触发 OPEN（`source_type = plain_field`）。

**Step 2: Run test to verify it fails**

Run:

```bash
cd desktop && pnpm test -- components/features/annotation/basic-info-form.test.tsx
```

Expected: FAIL（当前未渲染 `sop_id/name`）。

**Step 3: Write minimal implementation**

`basic-info-form.tsx` 调整：
- 构建字段列表时加入：
  - `{ key: "sop_id", label: "SOP ID", value: mockReviewData.sop.sop_id }`
  - `{ key: "sop_name", label: "SOP 名称", value: mockReviewData.sop.name }`
- 继续复用 `buildPlainLocation` 和 `OPEN` dispatch。
- 保留现有高亮逻辑。

**Step 4: Run test to verify it passes**

Run:

```bash
cd desktop && pnpm test -- components/features/annotation/basic-info-form.test.tsx
```

Expected: PASS。

**Step 5: Commit**

```bash
git add desktop/components/features/annotation/basic-info-form.tsx desktop/components/features/annotation/basic-info-form.test.tsx
git commit -m "feat(annotation): include sop base fields in basic info form"
```

---

### Task 6: 标注定位与高亮回归（SOP field_key）

**Files:**
- Modify: `desktop/components/features/annotation/quill-review-field-selection.test.tsx`
- Modify: `desktop/components/features/annotation/quill-review-field.test.tsx` (if needed)

**Step 1: Write the failing test**

在 `quill-review-field-selection.test.tsx` 增加 SOP 场景断言：
- 选中后 dispatch 的 location：
  - `panel = paragraph_detail`
  - `section_id = sop-step-<id>`
  - `field_key = sop.<id>.operation`（示例）
- submitted/draft 高亮调用顺序不变。

**Step 2: Run test to verify it fails**

Run:

```bash
cd desktop && pnpm test -- components/features/annotation/quill-review-field-selection.test.tsx
```

Expected: FAIL（旧测试未覆盖 SOP 编码/或断言不匹配新传参）。

**Step 3: Write minimal implementation**

如测试失败于 props 传递链路，最小修复对应映射或断言；不改 `QuillReviewField` 核心触发策略（300ms + OPEN）。

**Step 4: Run test to verify it passes**

Run:

```bash
cd desktop && pnpm test -- components/features/annotation/quill-review-field-selection.test.tsx
```

Expected: PASS。

**Step 5: Commit**

```bash
git add desktop/components/features/annotation/quill-review-field-selection.test.tsx desktop/components/features/annotation/quill-review-field.test.tsx
git commit -m "test(annotation): cover sop field_key location mapping"
```

---

### Task 7: 全量验证与收尾

**Files:**
- Modify (if needed): `docs/plans/2026-03-02-sop-annotation-frontend-implementation-plan.md`（仅在执行偏差时补充记录）

**Step 1: Run targeted test set**

```bash
cd desktop && pnpm test -- \
  components/features/annotation/sop-tree-nav.test.tsx \
  components/features/annotation/sop-step-detail.test.tsx \
  components/features/annotation/paragraph-panel.test.tsx \
  components/features/annotation/basic-info-form.test.tsx \
  components/features/annotation/quill-review-field-selection.test.tsx \
  components/features/annotation/right-annotation-panel.test.tsx
```

Expected: 全部 PASS。

**Step 2: Run lint for changed annotation files**

```bash
cd desktop && pnpm lint -- \
  components/features/annotation/sop-tree-nav.tsx \
  components/features/annotation/sop-step-detail.tsx \
  components/features/annotation/paragraph-panel.tsx \
  components/features/annotation/basic-info-form.tsx \
  components/features/annotation/quill-review-field.tsx
```

Expected: 无 error。

**Step 3: Run full verification gate**

```bash
cd desktop && pnpm lint && pnpm test && pnpm build
```

Expected: 全部命令 exit 0（仓库级历史 warning 可存在，但不能新增 error/fail）。

**Step 4: Final commit**

若 Task 7 产生变更，再提交：

```bash
git add <changed-files>
git commit -m "chore(annotation): finalize sop annotation frontend verification"
```

如无变更，跳过提交并记录“no-op commit”。

---

## 执行时输出模板（每任务）

- 改动文件：
- 测试命令：
- 测试结果：
- commit hash：

