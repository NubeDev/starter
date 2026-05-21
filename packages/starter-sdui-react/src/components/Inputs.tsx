/**
 * `field` / `select` / `toggle` — the three input primitives.
 *
 * All three integrate with `FormContext` (when nested under a
 * `form`) so values flow into the form's submit dispatch. Outside
 * a form they fall back to `pageState` writes via `useSdui`.
 *
 * Field-scoped diagnostics render inline below the control.
 */
import {
  Input,
  Label,
  Switch,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit";
import type { ComponentSpec } from "../registry/types.js";
import { useFormCtx } from "./form-context.js";
import { useSdui } from "../context.js";
import type { UiComponent, Diagnostic } from "../types.js";

function diagnosticsFor(name: string, list: Diagnostic[]): Diagnostic[] {
  return list.filter((d) => d.field === name);
}

function DiagnosticList({ items }: { items: Diagnostic[] }) {
  if (items.length === 0) return null;
  return (
    <div className="flex flex-col gap-0.5">
      {items.map((d, i) => (
        <div
          key={i}
          className={`text-xs ${
            d.severity === "error"
              ? "text-destructive"
              : d.severity === "warning"
              ? "text-amber-600"
              : "text-muted-foreground"
          }`}
        >
          {d.message}
        </div>
      ))}
    </div>
  );
}

export interface FieldNode extends UiComponent {
  type: "field";
  name: string;
  label?: string;
  placeholder?: string;
  input?: "text" | "number" | "email" | "password" | "url" | "tel";
  default_value?: string | number;
  required?: boolean;
  disabled?: boolean;
}

export const fieldSpec: ComponentSpec<FieldNode> = {
  kind: "field",
  Component: ({ node }) => {
    const form = useFormCtx();
    const { pageState, setPageState } = useSdui();
    const value =
      (form?.values[node.name] as string | number | undefined) ??
      (pageState[node.name] as string | number | undefined) ??
      node.default_value ??
      "";
    const onChange = (v: string) => {
      const cast = node.input === "number" ? (v === "" ? "" : Number(v)) : v;
      if (form) form.setField(node.name, cast);
      else setPageState({ [node.name]: cast });
    };
    const diags = diagnosticsFor(node.name, form?.diagnostics ?? []);
    const hasError = diags.some((d) => d.severity === "error");
    return (
      <div className="flex flex-col gap-1.5">
        {node.label ? <Label htmlFor={node.id ?? node.name}>{node.label}</Label> : null}
        <Input
          id={node.id ?? node.name}
          type={node.input ?? "text"}
          value={value as string | number}
          placeholder={node.placeholder}
          required={node.required}
          disabled={node.disabled}
          aria-invalid={hasError || undefined}
          onChange={(e) => onChange(e.target.value)}
        />
        <DiagnosticList items={diags} />
      </div>
    );
  },
};

export interface SelectOption {
  value: string;
  label: string;
}
export interface SelectNode extends UiComponent {
  type: "select";
  name: string;
  label?: string;
  options: SelectOption[];
  default_value?: string;
  placeholder?: string;
  disabled?: boolean;
}

export const selectSpec: ComponentSpec<SelectNode> = {
  kind: "select",
  Component: ({ node }) => {
    const form = useFormCtx();
    const { pageState, setPageState } = useSdui();
    const value =
      (form?.values[node.name] as string | undefined) ??
      (pageState[node.name] as string | undefined) ??
      node.default_value;
    const onValueChange = (v: string) => {
      if (form) form.setField(node.name, v);
      else setPageState({ [node.name]: v });
    };
    const diags = diagnosticsFor(node.name, form?.diagnostics ?? []);
    return (
      <div className="flex flex-col gap-1.5">
        {node.label ? <Label htmlFor={node.id ?? node.name}>{node.label}</Label> : null}
        <Select value={value} onValueChange={onValueChange} disabled={node.disabled}>
          <SelectTrigger id={node.id ?? node.name}>
            <SelectValue placeholder={node.placeholder ?? "Select…"} />
          </SelectTrigger>
          <SelectContent>
            {node.options.map((o) => (
              <SelectItem key={o.value} value={o.value}>
                {o.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <DiagnosticList items={diags} />
      </div>
    );
  },
};

export interface ToggleNode extends UiComponent {
  type: "toggle";
  name: string;
  label?: string;
  default_value?: boolean;
  disabled?: boolean;
}

export const toggleSpec: ComponentSpec<ToggleNode> = {
  kind: "toggle",
  Component: ({ node }) => {
    const form = useFormCtx();
    const { pageState, setPageState } = useSdui();
    const value =
      (form?.values[node.name] as boolean | undefined) ??
      (pageState[node.name] as boolean | undefined) ??
      node.default_value ??
      false;
    const onCheckedChange = (v: boolean) => {
      if (form) form.setField(node.name, v);
      else setPageState({ [node.name]: v });
    };
    const diags = diagnosticsFor(node.name, form?.diagnostics ?? []);
    return (
      <div className="flex flex-col gap-1.5">
        <div className="flex items-center gap-2">
          <Switch
            id={node.id ?? node.name}
            checked={value}
            disabled={node.disabled}
            onCheckedChange={onCheckedChange}
          />
          {node.label ? <Label htmlFor={node.id ?? node.name}>{node.label}</Label> : null}
        </div>
        <DiagnosticList items={diags} />
      </div>
    );
  },
};
