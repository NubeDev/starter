import { Plus, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Label } from "@nube/starter-ui-kit/components/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from "@nube/starter-ui-kit/components/select";

import type { Transform } from "@/data/types";
import type { EditorDraft } from "@/features/canvas/PanelEditor/useEditorDraft";
import { TransformRow } from "@/features/canvas/PanelEditor/TransformRow";

// Transforms tab: an ordered, client-side pipeline applied to query rows
// before render (rename / calculated field / filter / group-by / reduce /
// organize). The pipeline runs in `features/canvas/transforms` after the
// fetch, so reordering or editing here re-renders the preview from cached
// rows without refetching. Writes `config.transforms`.
const KINDS: ReadonlyArray<{ kind: Transform["kind"]; label: string }> = [
  { kind: "rename", label: "Rename field" },
  { kind: "calculated", label: "Calculated field" },
  { kind: "filter", label: "Filter rows" },
  { kind: "groupBy", label: "Group by" },
  { kind: "reduce", label: "Reduce" },
  { kind: "organize", label: "Organize fields" },
];

// A fresh transform of the chosen kind with empty-but-valid config so it
// is a no-op until the user fills it in (never fabricates data: an empty
// rename/filter passes rows through unchanged).
function blankTransform(kind: Transform["kind"]): Transform {
  switch (kind) {
    case "rename":
      return { kind, from: "", to: "" };
    case "calculated":
      return { kind, field: "", left: "", op: "+", right: "" };
    case "filter":
      return { kind, field: "", op: "=", value: "" };
    case "groupBy":
      return { kind, by: "", field: "", agg: "sum", as: "" };
    case "reduce":
      return { kind, field: "", calc: "last", as: "" };
    case "organize":
      return { kind, order: [] };
  }
}

export function TransformsTab({ draft }: { draft: EditorDraft }) {
  const { widget, patchConfig } = draft;
  const transforms = widget.config.transforms ?? [];

  function set(next: Transform[]) {
    patchConfig({ transforms: next.length > 0 ? next : undefined });
  }
  function update(i: number, next: Transform) {
    set(transforms.map((t, j) => (j === i ? next : t)));
  }
  function add(kind: Transform["kind"]) {
    set([...transforms, blankTransform(kind)]);
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <p className="text-xs text-muted-foreground">
          Reshape rows before render. Steps run top to bottom.
        </p>
        <Select value="" onValueChange={(v) => add(v as Transform["kind"])}>
          <SelectTrigger className="h-7 w-40 px-2 text-xs" aria-label="Add transform">
            <span className="flex items-center gap-1">
              <Plus className="size-3.5" /> Add transform
            </span>
          </SelectTrigger>
          <SelectContent>
            {KINDS.map((k) => (
              <SelectItem key={k.kind} value={k.kind}>
                {k.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {transforms.length === 0 ? (
        <p className="text-xs text-muted-foreground">No transforms yet.</p>
      ) : null}

      {transforms.map((t, i) => (
        <div key={i} className="space-y-2 rounded-lg border border-border/60 p-3">
          <div className="flex items-center justify-between">
            <Label className="text-xs uppercase tracking-wide text-muted-foreground">
              {KINDS.find((k) => k.kind === t.kind)?.label ?? t.kind}
            </Label>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label={`Remove transform ${i + 1}`}
              className="size-7 text-muted-foreground hover:text-destructive"
              onClick={() => set(transforms.filter((_, j) => j !== i))}
            >
              <Trash2 className="size-4" />
            </Button>
          </div>
          <TransformRow index={i} transform={t} onChange={(next) => update(i, next)} />
        </div>
      ))}
    </div>
  );
}
