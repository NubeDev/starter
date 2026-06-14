import type { SettingSpec, SettingValues } from "@/types";
import { Field, TextInput, Select } from "@/components/ui";

export function SettingsEditor({
  specs,
  values,
  onChange,
}: {
  specs: SettingSpec[];
  values: SettingValues;
  onChange: (next: SettingValues) => void;
}) {
  if (specs.length === 0) return <p className="text-xs text-muted">No configurable settings.</p>;

  const set = (k: string, v: string | number | boolean) => onChange({ ...values, [k]: v });

  return (
    <div className="grid grid-cols-2 gap-3">
      {specs.map((s) => {
        const val = values[s.key] ?? s.default ?? "";
        return (
          <Field key={s.key} label={s.unit ? `${s.label} (${s.unit})` : s.label}>
            {s.type === "select" ? (
              <Select value={String(val)} onChange={(e) => set(s.key, e.target.value)}>
                {(s.options ?? []).map((o) => (
                  <option key={o} value={o}>
                    {o}
                  </option>
                ))}
              </Select>
            ) : s.type === "bool" ? (
              <Select value={String(val)} onChange={(e) => set(s.key, e.target.value === "true")}>
                <option value="true">Yes</option>
                <option value="false">No</option>
              </Select>
            ) : (
              <TextInput
                type={s.type === "number" ? "number" : "text"}
                value={String(val)}
                placeholder={s.help}
                onChange={(e) => set(s.key, s.type === "number" ? Number(e.target.value) : e.target.value)}
              />
            )}
          </Field>
        );
      })}
    </div>
  );
}
