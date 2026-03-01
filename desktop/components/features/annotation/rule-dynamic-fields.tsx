import type { RuleFieldSchema } from "@/lib/annotation/types";

type RuleDynamicFieldsProps = {
  schema: RuleFieldSchema[];
  values: Record<string, string>;
  onChange: (key: string, value: string) => void;
};

export function RuleDynamicFields({ schema, values, onChange }: RuleDynamicFieldsProps) {
  return (
    <div className="space-y-3">
      {schema.map((field) => {
        const id = `rule-field-${field.key}`;
        const value = values[field.key] ?? "";

        if (field.type === "textarea") {
          return (
            <div key={field.key} className="space-y-1">
              <label htmlFor={id} className="block text-sm font-medium">
                {field.label}
              </label>
              <textarea
                id={id}
                className="min-h-[80px] w-full rounded-md border px-3 py-2 text-sm"
                required={field.required}
                value={value}
                onChange={(event) => onChange(field.key, event.target.value)}
              />
            </div>
          );
        }

        if (field.type === "select") {
          return (
            <div key={field.key} className="space-y-1">
              <label htmlFor={id} className="block text-sm font-medium">
                {field.label}
              </label>
              <select
                id={id}
                className="w-full rounded-md border px-3 py-2 text-sm"
                required={field.required}
                value={value}
                onChange={(event) => onChange(field.key, event.target.value)}
              >
                <option value="">请选择</option>
                {(field.options ?? []).map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </div>
          );
        }

        return (
          <div key={field.key} className="space-y-1">
            <label htmlFor={id} className="block text-sm font-medium">
              {field.label}
            </label>
            <input
              id={id}
              className="w-full rounded-md border px-3 py-2 text-sm"
              required={field.required}
              type="text"
              value={value}
              onChange={(event) => onChange(field.key, event.target.value)}
            />
          </div>
        );
      })}
    </div>
  );
}
