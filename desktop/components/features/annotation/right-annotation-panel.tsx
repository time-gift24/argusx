"use client";

import * as React from "react";
import { fallbackRules } from "@/lib/annotation/rules-fallback";
import { useAnnotationStore } from "@/lib/stores/annotation-store";
import { RuleDynamicFields } from "./rule-dynamic-fields";

export function RightAnnotationPanel() {
  const state = useAnnotationStore((store) => store.state);
  const dispatch = useAnnotationStore((store) => store.dispatch);
  const [isOpen, setIsOpen] = React.useState(false);
  const [selectedRuleCode, setSelectedRuleCode] = React.useState("");
  const [payload, setPayload] = React.useState<Record<string, string>>({});

  const active = state.items.find((item) => item.id === state.activeId);
  const selectedRule = fallbackRules.find((rule) => rule.code === selectedRuleCode);

  const isSubmitDisabled = !selectedRule || selectedRule.schema.some((field) => {
    if (!field.required) return false;
    return !(payload[field.key] ?? "").trim();
  });

  function handleSelectRule(code: string) {
    setSelectedRuleCode(code);
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
          className="w-full rounded-md border bg-background px-3 py-2 text-left text-sm"
          onClick={() => setIsOpen((prev) => !prev)}
        >
          {selectedRule?.label ?? "请选择"}
        </button>
        {isOpen ? (
          <ul className="rounded-md border bg-background" role="listbox" aria-label="违规检查项候选">
            {fallbackRules.map((rule) => (
              <li
                key={rule.code}
                role="option"
                aria-selected={selectedRuleCode === rule.code}
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
          values={payload}
          onChange={(key, value) => {
            setPayload((prev) => ({ ...prev, [key]: value }));
            dispatch({ type: "UPDATE_PAYLOAD", payload: { [key]: value } });
          }}
        />
      ) : null}

      <button
        type="button"
        className="rounded-md border px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-50"
        disabled={isSubmitDisabled}
      >
        提交标注
      </button>
    </div>
  );
}
