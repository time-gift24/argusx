import { QuillReviewField } from "@/components/features/annotation/quill-review-field";
import {
  buildSopFieldKey,
  buildSopSectionId,
  type SopSection,
} from "@/lib/annotation/sop-view-model";

type SopStepDetailData = Partial<Record<SopSection, string>>;

type SopStepDetailProps = {
  stepId: number;
  stepName: string;
  stepDetail: SopStepDetailData;
};

const SECTION_CONFIG: Array<{
  section: SopSection;
  label: string;
}> = [
  { section: "operation", label: "操作步骤" },
  { section: "verification", label: "操作验证" },
  { section: "impact_analysis", label: "影响分析" },
  { section: "rollback", label: "操作回退" },
];

export function SopStepDetail({
  stepId,
  stepName,
  stepDetail,
}: SopStepDetailProps) {
  const sectionId = buildSopSectionId(stepId);

  return (
    <div className="space-y-3 rounded-md border bg-background p-3">
      <h3 className="text-sm font-semibold">{stepName}</h3>

      {SECTION_CONFIG.map((item, index) => {
        const fieldKey = buildSopFieldKey(stepId, item.section);
        return (
          <details
            key={item.section}
            open={index === 0}
            className="rounded-md border bg-muted/20 p-2"
          >
            <summary className="cursor-pointer text-xs font-medium text-muted-foreground">
              {item.label}
            </summary>
            <div className="mt-2">
              <QuillReviewField
                sectionId={sectionId}
                fieldKey={fieldKey}
                nodeId={`${fieldKey}-node`}
                label={item.label}
                text={stepDetail[item.section] ?? ""}
              />
            </div>
          </details>
        );
      })}
    </div>
  );
}
