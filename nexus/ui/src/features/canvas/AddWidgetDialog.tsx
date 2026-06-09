import { useEffect, useState, type FormEvent } from "react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";
import type {
  Dashboard,
  Widget,
  WidgetLayout,
  WidgetType,
} from "@/data/types";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { SqlEditor } from "@/features/sql-editor";
import { nextSlot } from "@/features/canvas/placement";
import { useAddPanel } from "@/features/dashboards/useAddPanel";
import { WIDGET_CATALOG, WIDGET_TYPES } from "@/features/widgets/catalog";

// Picker order and per-type metadata (label, default footprint, whether
// an x column is needed) come from the widget catalog — the one place a
// panel type is declared. Adding a type to the catalog lists it here for
// free with the right size and x-column prompt.
const needsX = (t: WidgetType) => WIDGET_CATALOG[t].roles.x === "required";

// Builds a draft panel — type, datasource, SQL, and the field mapping
// (which column is the x axis, which is the value) — and adds it to the
// dashboard via `POST /panels`. The field mapping is authored here because
// the backend doesn't model it; it rides in the opaque layout (D-adapter).
export function AddWidgetDialog({
  dashboard,
  open,
  onOpenChange,
  initial,
}: {
  dashboard: Dashboard;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** When the dialog is opened by a palette drop, seed the type and pin
   *  the panel to the dropped grid cell instead of auto-placing it. */
  initial?: { type?: WidgetType; position?: WidgetLayout };
}) {
  const add = useAddPanel(dashboard.slug);
  const [type, setType] = useState<WidgetType>("line");
  const [title, setTitle] = useState("");
  const [datasourceId, setDatasourceId] = useState<string | undefined>();
  const [sql, setSql] = useState("");
  const [xCol, setXCol] = useState("");
  const [valueCol, setValueCol] = useState("");

  // Adopt the dropped type when the dialog is opened from the palette.
  // Keyed on open + the seed type so reopening with a different tile
  // re-seeds, but typing a different type by hand mid-edit isn't clobbered.
  const seedType = initial?.type;
  useEffect(() => {
    if (open && seedType) setType(seedType);
  }, [open, seedType]);

  const ready = title.trim() && datasourceId && sql.trim() && valueCol.trim();

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!ready) return;
    const size = WIDGET_CATALOG[type].defaultSize;
    const draft: Widget = {
      id: "",
      type,
      title: title.trim(),
      // A drop pins the panel to the cell it landed on (clamped to the
      // type's footprint); the toolbar button auto-places at the bottom.
      layout: initial?.position
        ? { ...initial.position, w: size.w, h: size.h }
        : nextSlot(dashboard.widgets, size.w, size.h),
      config: {
        query: { datasourceId: datasourceId!, sql: sql.trim() },
        fields: {
          x: needsX(type) ? xCol.trim() || undefined : undefined,
          series: [{ value: valueCol.trim() }],
        },
      },
    };
    add.mutate(draft, { onSuccess: () => onOpenChange(false) });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass max-w-lg">
        <DialogHeader>
          <DialogTitle>Add panel</DialogTitle>
          <DialogDescription>
            Pick a visualization and the query that feeds it.
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-4" onSubmit={onSubmit}>
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="panel-type">Type</Label>
              <Select value={type} onValueChange={(v) => setType(v as WidgetType)}>
                <SelectTrigger id="panel-type">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {WIDGET_TYPES.map((t) => (
                    <SelectItem key={t} value={t}>
                      {WIDGET_CATALOG[t].label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="panel-title">Title</Label>
              <Input
                id="panel-title"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                required
              />
            </div>
          </div>
          <div className="space-y-2">
            <Label>Datasource</Label>
            <DatasourcePicker value={datasourceId} onChange={setDatasourceId} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="panel-sql">SQL</Label>
            <SqlEditor
              id="panel-sql"
              value={sql}
              onChange={setSql}
              datasourceId={datasourceId}
              minHeight="6rem"
              placeholder="select … from … limit 100"
              ariaLabel="Panel SQL"
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            {needsX(type) ? (
              <div className="space-y-2">
                <Label htmlFor="panel-x">X column</Label>
                <Input
                  id="panel-x"
                  value={xCol}
                  onChange={(e) => setXCol(e.target.value)}
                  placeholder="ts"
                />
              </div>
            ) : null}
            <div className="space-y-2">
              <Label htmlFor="panel-value">Value column</Label>
              <Input
                id="panel-value"
                value={valueCol}
                onChange={(e) => setValueCol(e.target.value)}
                placeholder="value"
                required
              />
            </div>
          </div>
          {add.isError ? (
            <p role="alert" className="text-sm text-destructive">
              Couldn't add the panel.
            </p>
          ) : null}
          <DialogFooter>
            <Button type="submit" disabled={!ready || add.isPending}>
              {add.isPending ? "Adding…" : "Add panel"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
