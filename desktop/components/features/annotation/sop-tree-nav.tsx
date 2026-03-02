import type { SopCategory, SopGroups } from "@/lib/annotation/sop-view-model";

type SopTreeNavProps = {
  groups: SopGroups;
  activeStepId: number | null;
  onSelect: (stepId: number) => void;
};

const GROUPS: Array<{
  category: SopCategory;
  label: string;
}> = [
  { category: "detect", label: "01操作检测" },
  { category: "handle", label: "02操作处理" },
  { category: "verification", label: "03操作验证" },
  { category: "rollback", label: "04操作回退" },
];

export function SopTreeNav({ groups, activeStepId, onSelect }: SopTreeNavProps) {
  return (
    <div className="space-y-2">
      {GROUPS.map((group) => {
        const steps = groups[group.category];
        return (
          <details key={group.category} open className="rounded-md border bg-background p-2">
            <summary className="cursor-pointer text-sm font-medium">
              {group.label}
            </summary>
            <div className="mt-2 space-y-1">
              {steps.length === 0 ? (
                <p className="px-2 py-1 text-xs text-muted-foreground">暂无步骤</p>
              ) : (
                steps.map((step) => (
                  <button
                    key={step.sop_step_id}
                    type="button"
                    className={`w-full rounded-md border px-2 py-1 text-left text-sm ${
                      step.sop_step_id === activeStepId
                        ? "border-border bg-muted"
                        : "border-border/60 bg-background hover:bg-muted/50"
                    }`}
                    onClick={() => onSelect(step.sop_step_id)}
                  >
                    {step.name}
                  </button>
                ))
              )}
            </div>
          </details>
        );
      })}
    </div>
  );
}
