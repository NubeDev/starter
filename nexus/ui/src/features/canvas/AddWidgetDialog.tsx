import { useState, type FormEvent } from "react";
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
import { Textarea } from "@nube/starter-ui-kit/components/textarea";

import type { Dashboard, Widget, WidgetType } from "@/data/types";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { nextSlot } from "@/features/canvas/placement";
import { useAddPanel } from "@/features/dashboards/useAddPanel";

const TYPES: WidgetType[] = ["line", "area", "gauge", "stat", "status", "table"];
const NEEDS_X = new Set<WidgetType>(["line", "area"]);

// Default footprint per type, mirroring the canvas min sizes.
const SIZE: Record<WidgetType, { w: number; h: number }> = {
  stat: { w: 3, h: 2 },
  gauge: { w: 3, h: 3 },
  line: { w: 6, h: 4 },
  area: { w: 6, h: 4 },
  status: { w: 3, h: 4 },
  table: { w: 6, h: 4 },
};

// Builds a draft panel — type, datasource, SQL, and the field mapping
// (which column is the x axis, which is the value) — and adds it to the
// dashboard via `POST /panels`. The field mapping is authored here because
// the backend doesn't model it; it rides in the opaque layout (D-adapter).
export function AddWidgetDialog({
  dashboard,
  open,
  onOpenChange,
}: {
  dashboard: Dashboard;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const add = useAddPanel(dashboard.slug);
  const [type, setType] = useState<WidgetType>("line");
  const [title, setTitle] = useState("");
  const [datasourceId, setDatasourceId] = useState<string | undefined>();
  const [sql, setSql] = useState("");
  const [xCol, setXCol] = useState("");
  const [valueCol, setValueCol] = useState("");

  const ready = title.trim() && datasourceId && sql.trim() && valueCol.trim();

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!ready) return;
    const size = SIZE[type];
    const draft: Widget = {
      id: "",
      type,
      title: title.trim(),
      layout: nextSlot(dashboard.widgets, size.w, size.h),
      config: {
        query: { datasourceId: datasourceId!, sql: sql.trim() },
        fields: {
          x: NEEDS_X.has(type) ? xCol.trim() || undefined : undefined,
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
                  {TYPES.map((t) => (
                    <SelectItem key={t} value={t} className="capitalize">
                      {t}
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
            <Textarea
              id="panel-sql"
              value={sql}
              onChange={(e) => setSql(e.target.value)}
              placeholder="select … from … limit 100"
              spellCheck={false}
              className="min-h-24 resize-y font-mono text-sm"
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            {NEEDS_X.has(type) ? (
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
