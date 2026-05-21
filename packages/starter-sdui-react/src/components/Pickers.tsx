/**
 * `ref_picker` and `date_range` — two two-way inputs that write to
 * `$page` (date_range) or to a form context (ref_picker).
 *
 * `ref_picker` is a node-graph reference selector. The RSQL `query`
 * narrows the candidate set the host's picker offers; the picked
 * id flows through the form coordinator like any other field. The
 * actual graph search dialog is host-side — this wrapper renders a
 * placeholder + the current id, matching the IR shape.
 *
 * `date_range` writes `{ from, to }` (Unix ms) into
 * `$page[page_state_key]` on every preset click. `from`/`to: null`
 * means "all time" (unbounded). Downstream `chart` nodes reading
 * the same `page_state_key` retune automatically via the
 * re-resolve round-trip.
 */
import { Input, Label } from "@nube/starter-ui-kit";
import type { ComponentSpec } from "../registry/types.js";
import type { UiComponent } from "../types.js";
import { useSdui } from "../context.js";
import { useFormCtx } from "./form-context.js";

export interface RefPickerNode extends UiComponent {
  type: "ref_picker";
  name?: string;
  query?: string;
  value?: string;
  placeholder?: string;
  label?: string;
}

export const refPickerSpec: ComponentSpec<RefPickerNode> = {
  kind: "ref_picker" as never,
  Component: ({ node }) => {
    const form = useFormCtx();
    const { pageState, setPageState } = useSdui();
    const fieldName = node.name ?? node.id ?? "ref";
    const current =
      (form?.values[fieldName] as string | undefined) ??
      (pageState[fieldName] as string | undefined) ??
      node.value ??
      "";
    const onChange = (v: string) => {
      if (form) form.setField(fieldName, v);
      else setPageState({ [fieldName]: v });
    };
    return (
      <div className="flex flex-col gap-1.5">
        {node.label ? <Label htmlFor={fieldName}>{node.label}</Label> : null}
        <Input
          id={fieldName}
          value={current}
          placeholder={node.placeholder ?? "node id…"}
          data-query={node.query}
          onChange={(e) => onChange(e.target.value)}
        />
      </div>
    );
  },
};

export interface DateRangePreset {
  label: string;
  duration_ms: number | null;
}
export interface DateRangeNode extends UiComponent {
  type: "date_range";
  page_state_key: string;
  presets?: DateRangePreset[];
}

export const dateRangeSpec: ComponentSpec<DateRangeNode> = {
  kind: "date_range" as never,
  Component: ({ node }) => {
    const { pageState, setPageState } = useSdui();
    const current = pageState[node.page_state_key] as
      | { from: number | null; to: number | null }
      | undefined;
    const presets = node.presets ?? [];
    const apply = (p: DateRangePreset) => {
      if (p.duration_ms === null) {
        setPageState({ [node.page_state_key]: { from: null, to: null } });
        return;
      }
      const to = Date.now();
      const from = to - p.duration_ms;
      setPageState({ [node.page_state_key]: { from, to } });
    };
    return (
      <div className={`flex flex-wrap items-center gap-1 ${node.style?.className ?? ""}`}>
        {presets.map((p, i) => (
          <button
            key={i}
            type="button"
            className="rounded border px-2 py-1 text-xs hover:bg-accent"
            onClick={() => apply(p)}
          >
            {p.label}
          </button>
        ))}
        <span className="text-xs text-muted-foreground">
          from {current?.from ?? "—"} to {current?.to ?? "—"}
        </span>
      </div>
    );
  },
};
