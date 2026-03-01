import { BasicInfoForm } from "./basic-info-form";
import { ParagraphPanel } from "./paragraph-panel";

export function LeftReviewPane() {
  return (
    <section data-testid="review-left-pane" className="space-y-4 rounded-md border bg-background p-4">
      <details open>
        <summary className="cursor-pointer text-sm font-semibold">基础信息</summary>
        <div className="mt-3">
          <BasicInfoForm />
        </div>
      </details>

      <details open>
        <summary className="cursor-pointer text-sm font-semibold">段落明细</summary>
        <div className="mt-3">
          <ParagraphPanel />
        </div>
      </details>
    </section>
  );
}
