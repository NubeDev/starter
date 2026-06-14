// Pick a component type from a catalog and edit its fields.

import type { ComponentKind } from "../api/catalog";
import type { Picked } from "../builder/assemble";
import { FieldInput } from "./FieldInput";

interface Props {
  title: string;
  kinds: ComponentKind[];
  picked: Picked | null;
  onChange: (picked: Picked | null) => void;
  optional?: boolean;
}

export function ComponentForm({ title, kinds, picked, onChange, optional }: Props) {
  function selectType(type: string) {
    if (type === "") return onChange(null);
    const kind = kinds.find((k) => k.type === type);
    if (kind) onChange({ kind, values: {} });
  }

  function setValue(name: string, value: string) {
    if (!picked) return;
    onChange({ kind: picked.kind, values: { ...picked.values, [name]: value } });
  }

  return (
    <section className="card">
      <header className="card-head">
        <h3>{title}</h3>
        <select value={picked?.kind.type ?? ""} onChange={(e) => selectType(e.target.value)}>
          {optional && <option value="">— none —</option>}
          {!optional && !picked && <option value="">select…</option>}
          {kinds.map((k) => (
            <option key={k.type} value={k.type}>
              {k.label}
            </option>
          ))}
        </select>
      </header>
      {picked && (
        <>
          <p className="summary">{picked.kind.summary}</p>
          {picked.kind.fields.map((field) => (
            <FieldInput
              key={field.name}
              field={field}
              value={picked.values[field.name] ?? ""}
              onChange={(v) => setValue(field.name, v)}
            />
          ))}
        </>
      )}
    </section>
  );
}
