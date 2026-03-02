import { useMemo, useState } from "react";
import { mockReviewData } from "./mock-review-data";
import { SopStepDetail } from "./sop-step-detail";
import { SopTreeNav } from "./sop-tree-nav";
import {
  pickDefaultSopStep,
  type SopGroups,
  type SopStepLite,
} from "@/lib/annotation/sop-view-model";

function findStepById(groups: SopGroups, stepId: number): SopStepLite | null {
  for (const category of ["detect", "handle", "verification", "rollback"] as const) {
    const hit = groups[category].find((step) => step.sop_step_id === stepId);
    if (hit) {
      return hit;
    }
  }

  return null;
}

export function ParagraphPanel() {
  const groups = useMemo<SopGroups>(() => ({
    detect: mockReviewData.sop?.detect ?? [],
    handle: mockReviewData.sop?.handle ?? [],
    verification: mockReviewData.sop?.verification ?? [],
    rollback: mockReviewData.sop?.rollback ?? [],
  }), []);

  const [activeStepId, setActiveStepId] = useState<number | null>(() => pickDefaultSopStep(groups)?.sop_step_id ?? null);

  const activeStep = activeStepId === null ? null : findStepById(groups, activeStepId);
  const activeDetail = activeStepId === null ? null : mockReviewData.sop?.step_details?.[activeStepId];

  if (!activeStep || !activeDetail) {
    return (
      <div className="rounded-md border bg-background p-3 text-sm text-muted-foreground">
        暂无可展示的 SOP 步骤。
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-3 lg:grid-cols-[260px_minmax(0,1fr)]">
      <div>
        <SopTreeNav
          groups={groups}
          activeStepId={activeStepId}
          onSelect={setActiveStepId}
        />
      </div>
      <SopStepDetail
        stepId={activeStep.sop_step_id}
        stepName={activeStep.name}
        stepDetail={activeDetail}
      />
    </div>
  );
}
