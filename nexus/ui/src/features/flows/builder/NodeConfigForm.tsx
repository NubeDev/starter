import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
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
};

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
        const value = current == null ? "" : String(current);
        const id = `cfg-${key}`;
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
