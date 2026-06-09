// Render one editable input for a catalog Field, by its kind.

import type { Field } from "../api/catalog";

interface Props {
  field: Field;
  value: string;
  onChange: (value: string) => void;
}

export function FieldInput({ field, value, onChange }: Props) {
  const multiline = field.kind === "code" || field.kind === "list";
  const label = (
    <span className="field-label">
      {field.name}
      {field.required && <em className="req">*</em>}
      <span className="field-kind">{field.kind}</span>
    </span>
  );

  return (
    <label className="field">
      {label}
      {multiline ? (
        <textarea
          rows={field.kind === "code" ? 4 : 2}
          placeholder={field.placeholder ?? ""}
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
      ) : field.kind === "bool" ? (
        <select value={value || "false"} onChange={(e) => onChange(e.target.value)}>
          <option value="false">false</option>
          <option value="true">true</option>
        </select>
      ) : (
        <input
          type={field.kind === "number" ? "number" : "text"}
          placeholder={field.placeholder ?? ""}
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
      )}
      {field.help && <span className="field-help">{field.help}</span>}
    </label>
  );
}
