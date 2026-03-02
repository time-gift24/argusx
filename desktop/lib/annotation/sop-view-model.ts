export type SopCategory = "detect" | "handle" | "verification" | "rollback";

export type SopSection =
  | "operation"
  | "verification"
  | "impact_analysis"
  | "rollback";

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
  const order: SopCategory[] = [
    "detect",
    "handle",
    "verification",
    "rollback",
  ];

  for (const category of order) {
    if (groups[category].length > 0) {
      return groups[category][0];
    }
  }

  return null;
}
