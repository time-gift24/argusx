"use client";

import * as React from "react";
import { Button } from "@/components/ui/button";
import { fallbackRules } from "@/lib/annotation/rules-fallback";
import { useAnnotationStore } from "@/lib/stores/annotation-store";
import { toast } from "sonner";
import { RuleDynamicFields } from "./rule-dynamic-fields";

function LocationReadonlyField({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <div className="space-y-1">
      <label className="block text-xs text-muted-foreground">{label}</label>
      <input
        readOnly
        className="w-full rounded-md border bg-background px-3 py-2 text-sm"
        value={value}
      />
    </div>
  );
}

export function RightAnnotationPanel() {
  const state = useAnnotationStore((store) => store.state);
  const catalog = useAnnotationStore((store) => store.catalog);
  const dispatch = useAnnotationStore((store) => store.dispatch);
  const submitActive = useAnnotationStore((store) => store.submitActive);
  const listboxId = React.useId();
  const [isOpen, setIsOpen] = React.useState(false);
  const [isSubmitting, setIsSubmitting] = React.useState(false);

  const active = state.items.find((item) => item.id === state.activeId);
  const hasActiveTarget = Boolean(active);
  const rules = catalog.length > 0 ? catalog : fallbackRules;
  const effectiveRuleCode = active?.ruleCode ?? "";
  const selectedRule = rules.find((rule) => rule.code === effectiveRuleCode);
  const effectivePayload = active?.payload ?? {};
  const location = active?.location;
  const isRichSelection = location?.source_type === "rich_text_selection";
  const isSubmitted = active?.status === "submitted";

  const isSubmitDisabled = isSubmitting
    || isSubmitted
    || !hasActiveTarget
    || !selectedRule
    || selectedRule.schema.some((field) => {
      if (!field.required) return false;
      return !(effectivePayload[field.key] ?? "").trim();
    });

  React.useEffect(() => {
    setIsOpen(false);
    setIsSubmitting(false);
  }, [state.activeId]);

  function handleSelectRule(code: string) {
    if (!hasActiveTarget) {
      return;
    }
    setIsOpen(false);
    dispatch({ type: "UPDATE_RULE", ruleCode: code });
  }

  async function handleSubmit() {
    if (isSubmitDisabled) {
      return;
    }

    try {
      setIsSubmitting(true);
      await submitActive();
      toast.success("标注已提交");
    } catch {
      toast.error("提交失败，请稍后重试");
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <div data-testid="annotation-right-panel" className="space-y-4 rounded-md border bg-muted/20 p-4">
      <h2 className="text-sm font-semibold">标注面板</h2>

      <details className="rounded-md border bg-background/70 p-3">
        <summary className="cursor-pointer text-xs font-medium text-muted-foreground">
          定位信息（只读）
        </summary>
        <div className="mt-3 space-y-2">
          <LocationReadonlyField label="来源类型" value={location?.source_type ?? ""} />
          <LocationReadonlyField label="面板" value={location?.panel ?? ""} />
          <LocationReadonlyField label="区段 ID" value={location?.section_id ?? ""} />
          <LocationReadonlyField label="定位字段" value={location?.field_key ?? ""} />
          <LocationReadonlyField label="节点 ID" value={location?.node_id ?? ""} />
          {isRichSelection ? (
            <>
              <LocationReadonlyField
                label="起始偏移"
                value={location?.start_offset !== null && location?.start_offset !== undefined ? String(location.start_offset) : ""}
              />
              <LocationReadonlyField
                label="结束偏移"
                value={location?.end_offset !== null && location?.end_offset !== undefined ? String(location.end_offset) : ""}
              />
              <div className="space-y-1">
                <label className="block text-xs text-muted-foreground">选中文本</label>
                <textarea
                  readOnly
                  className="min-h-[64px] w-full rounded-md border bg-background px-3 py-2 text-sm"
                  value={location?.selected_text ?? ""}
                />
              </div>
            </>
          ) : null}
        </div>
      </details>

      <div className="space-y-1">
        <label className="block text-sm font-medium" htmlFor="rule-combobox">
          违规检查项
        </label>
        <button
          id="rule-combobox"
          type="button"
          role="combobox"
          aria-label="违规检查项"
          aria-expanded={isOpen}
          aria-controls={listboxId}
          disabled={!hasActiveTarget}
          className="w-full rounded-md border bg-background px-3 py-2 text-left text-sm"
          onClick={() => setIsOpen((prev) => !prev)}
        >
          {selectedRule?.label ?? "请选择"}
        </button>
        {!hasActiveTarget ? (
          <p className="text-xs text-muted-foreground">请选择左侧内容开始标注。</p>
        ) : null}
        {isOpen ? (
          <ul
            id={listboxId}
            className="rounded-md border bg-background"
            role="listbox"
            aria-label="违规检查项候选"
          >
            {rules.map((rule) => (
              <li
                key={rule.code}
                role="option"
                aria-selected={effectiveRuleCode === rule.code}
                className="cursor-pointer px-3 py-2 text-sm hover:bg-accent"
                onClick={() => handleSelectRule(rule.code)}
              >
                {rule.label}
              </li>
            ))}
          </ul>
        ) : null}
      </div>

      {selectedRule ? (
        <RuleDynamicFields
          schema={selectedRule.schema}
          values={effectivePayload}
          onChange={(key, value) => {
            if (!hasActiveTarget) {
              return;
            }
            dispatch({
              type: "UPDATE_PAYLOAD",
              payload: { [key]: value },
            });
          }}
        />
      ) : null}

      <Button
        type="button"
        size="lg"
        disabled={isSubmitDisabled}
        onClick={() => {
          void handleSubmit();
        }}
      >
        {isSubmitted ? "已提交" : isSubmitting ? "提交中..." : "提交标注"}
      </Button>
    </div>
  );
}
