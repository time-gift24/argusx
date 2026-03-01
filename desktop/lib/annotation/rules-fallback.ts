import type { RuleCatalogItem } from "./types";

export const fallbackRules: RuleCatalogItem[] = [
  {
    code: "FACT_CONSISTENCY",
    label: "事实一致性",
    description: "核对关键事实是否与原文一致。",
    version: 1,
    schema: [
      {
        key: "issue",
        label: "问题说明",
        type: "textarea",
        required: true,
      },
    ],
  },
];
