"use client";

import * as React from "react";
import { fallbackRules } from "@/lib/annotation/rules-fallback";
import { useAnnotationStore } from "@/lib/stores/annotation-store";
import { RuleDynamicFields } from "./rule-dynamic-fields";

export function RightAnnotationPanel() {
  const state = useAnnotationStore((store) => store.state);
  const catalog = useAnnotationStore((store) => store.catalog);
  const dispatch = useAnnotationStore((store) => store.dispatch);
  const submitActive = useAnnotationStore((store) => store.submitActive);
  const listboxId = React.useId();
  const [isOpen, setIsOpen] = React.useState(false);

  const active = state.items.find((item) => item.id === state.activeId);
  const hasActiveTarget = Boolean(active);
  const rules = catalog.length > 0 ? catalog : fallbackRules;
  const effectiveRuleCode = active?.ruleCode ?? "";
  const selectedRule = rules.find((rule) => rule.code === effectiveRuleCode);
  const effectivePayload = active?.payload ?? {};

  const isSubmitDisabled = !hasActiveTarget || !selectedRule || selectedRule.schema.some((field) => {
    if (!field.required) return false;
    return !(effectivePayload[field.key] ?? "").trim();
  });

  React.useEffect(() => {
    setIsOpen(false);
  }, [state.activeId]);

  function handleSelectRule(code: string) {
    if (!hasActiveTarget) {
      return;
    }
    setIsOpen(false);
    dispatch({ type: "UPDATE_RULE", ruleCode: code });
  }

  return (
    <div data-testid="annotation-right-panel" className="space-y-4 rounded-md border bg-muted/20 p-4">
      <h2 className="text-sm font-semibold">标注面板</h2>

      <div className="space-y-1">
        <label className="block text-xs text-muted-foreground">定位字段</label>
        <input
          readOnly
          className="w-full rounded-md border bg-background px-3 py-2 text-sm"
          value={active?.location.field_key ?? ""}
        />
      </div>

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

      <button
        type="button"
        className="rounded-md border px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-50"
        disabled={isSubmitDisabled}
        onClick={() => {
          void submitActive();
        }}
      >
        提交标注
      </button>
    </div>
  );
}
