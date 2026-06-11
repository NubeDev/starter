import { Plus, X } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
import { Switch } from "@nube/starter-ui-kit/components/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

// A config form generated from a node's JSON Schema, so a node is configured
// through typed fields rather than raw JSON. Deliberately covers the subset of
// JSON Schema the registered nodes use (object with string/integer/enum
// properties + a `required` list); a property the renderer doesn't recognise
// falls back to a text field so nothing is silently undroppable.

type JsonSchema = {
  type?: string;
  properties?: Record<string, PropertySchema>;
  required?: string[];
};

type PropertySchema = {
  type?: string;
  description?: string;
  enum?: string[];
  // Present for object/array properties — used to detect the declared-columns
  // shape (an object holding `fields: [{name, type, nullable}]`).
  properties?: Record<string, PropertySchema>;
  items?: PropertySchema;
};

// A declared column the FieldsEditor edits. Mirrors the backend
// `json_to_arrow` schema field: `{ name, type, nullable? }`.
type DeclaredField = { name: string; type: string; nullable?: boolean };

// Recognise the "declared columns" property: an object with a `fields` array of
// objects whose item schema offers a `type` enum. Generic — any node that
// declares this shape gets the editor, not just json_to_arrow.
function declaredColumnsTypes(prop: PropertySchema): string[] | null {
  const fields = prop.properties?.fields;
  if (prop.type !== "object" || fields?.type !== "array") return null;
  const typeEnum = fields.items?.properties?.type?.enum;
  return typeEnum && typeEnum.length > 0 ? typeEnum : null;
}

// Coerce a form string back to the schema's type so a numeric field stays a
// number on the wire (the backend deserialises by type).
function coerce(prop: PropertySchema, raw: string): unknown {
  if (prop.type === "integer" || prop.type === "number") {
    if (raw.trim() === "") return undefined;
    const n = Number(raw);
    return Number.isFinite(n) ? n : raw;
  }
  return raw === "" ? undefined : raw;
}

export function NodeConfigForm({
  schema,
  config,
  onChange,
  secretFields,
  emptyHint = "This node has no configuration.",
}: {
  schema: unknown;
  config: Record<string, unknown>;
  onChange: (config: Record<string, unknown>) => void;
  // Property names the schema declares as write-only secrets. Rendered as
  // password inputs. Shared with the datasource create form, which marks its
  // kind's `secret_fields` this way; flow nodes pass none.
  secretFields?: readonly string[];
  // Override for the no-properties placeholder so callers other than the flow
  // builder read naturally.
  emptyHint?: string;
}) {
  const s = (schema ?? {}) as JsonSchema;
  const props = s.properties ?? {};
  const required = new Set(s.required ?? []);
  const secrets = new Set(secretFields ?? []);
  const keys = Object.keys(props);

  if (keys.length === 0) {
    return <p className="text-xs text-muted-foreground">{emptyHint}</p>;
  }

  const set = (key: string, prop: PropertySchema, raw: string) => {
    const next = { ...config };
    const value = coerce(prop, raw);
    if (value === undefined) delete next[key];
    else next[key] = value;
    onChange(next);
  };

  return (
    <div className="flex flex-col gap-3">
      {keys.map((key) => {
        const prop = props[key];
        const current = config[key];
        const id = `cfg-${key}`;

        // A declared-columns property (e.g. json_to_arrow's `schema`) gets a
        // dedicated rows editor — the flat renderer can't draw array-of-objects.
        const columnTypes = declaredColumnsTypes(prop);
        if (columnTypes) {
          return (
            <DeclaredColumnsField
              key={key}
              label={key}
              description={prop.description}
              types={columnTypes}
              value={current}
              onChange={(next) => {
                const cfg = { ...config };
                if (next === undefined) delete cfg[key];
                else cfg[key] = next;
                onChange(cfg);
              }}
            />
          );
        }

        const value = current == null ? "" : String(current);
        return (
          <div key={key} className="space-y-1.5">
            <Label htmlFor={id} className="text-xs">
              {key}
              {required.has(key) ? <span className="text-destructive"> *</span> : null}
            </Label>
            {prop.enum ? (
              <Select value={value} onValueChange={(v) => set(key, prop, v)}>
                <SelectTrigger id={id} className="h-8">
                  <SelectValue placeholder="Select…" />
                </SelectTrigger>
                <SelectContent>
                  {prop.enum.map((opt) => (
                    <SelectItem key={opt} value={opt}>
                      {opt}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : (
              <Input
                id={id}
                value={value}
                type={
                  secrets.has(key)
                    ? "password"
                    : prop.type === "integer" || prop.type === "number"
                      ? "number"
                      : "text"
                }
                autoComplete={secrets.has(key) ? "new-password" : "off"}
                onChange={(e) => set(key, prop, e.target.value)}
                placeholder={prop.description}
                className="h-8"
              />
            )}
            {prop.description ? (
              <p className="text-[11px] leading-tight text-muted-foreground">
                {prop.description}
              </p>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

// Editor for an optional declared-columns config (`{ fields: [{name,type,nullable}] }`).
// Empty list → the property is omitted entirely, so the backend infers the
// schema from the first batch. Each row is a name + type dropdown + nullable
// toggle.
function DeclaredColumnsField({
  label,
  description,
  types,
  value,
  onChange,
}: {
  label: string;
  description?: string;
  types: string[];
  value: unknown;
  onChange: (next: { fields: DeclaredField[] } | undefined) => void;
}) {
  const fields: DeclaredField[] = Array.isArray(
    (value as { fields?: unknown } | undefined)?.fields,
  )
    ? ((value as { fields: DeclaredField[] }).fields)
    : [];

  // Write back, omitting the whole property when no columns are declared so an
  // empty editor means "infer", not "a schema with zero columns" (an error).
  const commit = (next: DeclaredField[]) =>
    onChange(next.length ? { fields: next } : undefined);

  const update = (i: number, patch: Partial<DeclaredField>) =>
    commit(fields.map((f, j) => (j === i ? { ...f, ...patch } : f)));
  const remove = (i: number) => commit(fields.filter((_, j) => j !== i));
  const add = () =>
    commit([...fields, { name: "", type: types[0] ?? "string", nullable: true }]);

  return (
    <div className="space-y-1.5">
      <Label className="text-xs">{label}</Label>
      {description ? (
        <p className="text-[11px] leading-tight text-muted-foreground">
          {description}
        </p>
      ) : null}

      {fields.length === 0 ? (
        <p className="rounded border border-dashed border-border/60 px-2 py-1.5 text-[11px] text-muted-foreground">
          No columns declared — the schema is inferred from the first batch. Add
          columns to pin types (recommended for a database sink).
        </p>
      ) : (
        <div className="flex flex-col gap-1.5">
          {fields.map((f, i) => (
            <div key={i} className="flex items-center gap-1.5">
              <Input
                value={f.name}
                onChange={(e) => update(i, { name: e.target.value })}
                placeholder="column name"
                className="h-8 flex-1"
              />
              <Select value={f.type} onValueChange={(v) => update(i, { type: v })}>
                <SelectTrigger className="h-8 w-28">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {types.map((t) => (
                    <SelectItem key={t} value={t}>
                      {t}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <label className="flex items-center gap-1 text-[11px] text-muted-foreground">
                <Switch
                  checked={f.nullable !== false}
                  onCheckedChange={(c) => update(i, { nullable: c })}
                />
                nullable
              </label>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-8 shrink-0"
                onClick={() => remove(i)}
                aria-label={`Remove column ${f.name || i + 1}`}
              >
                <X className="size-3.5" />
              </Button>
            </div>
          ))}
        </div>
      )}

      <Button type="button" variant="outline" size="sm" onClick={add}>
        <Plus className="size-3.5" aria-hidden />
        Add column
      </Button>
    </div>
  );
}
