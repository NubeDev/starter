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

import type { AxisOptions, LegendOptions, PanelOptions } from "@/data/types";
import type { EditorDraft } from "@/features/canvas/PanelEditor/useEditorDraft";

// Legend & Axes tab: chart chrome the cartesian builders (line/area/bar)
// read via `cartesianChrome` — legend on/off + placement, and the y-axis
// scale (linear/log), soft bounds, and label. Writes `config.options`.
// Single-value panels (stat/gauge) ignore these, so absence keeps their
// behaviour exactly as before.
export function LegendAxesTab({ draft }: { draft: EditorDraft }) {
  const { widget, patchConfig } = draft;
  const options: PanelOptions = widget.config.options ?? {};
  const legend: LegendOptions = options.legend ?? {};
  const yAxis: AxisOptions = options.yAxis ?? {};

  function setLegend(patch: Partial<LegendOptions>) {
    patchConfig({ options: { ...options, legend: { ...legend, ...patch } } });
  }
  function setAxis(patch: Partial<AxisOptions>) {
    patchConfig({ options: { ...options, yAxis: { ...yAxis, ...patch } } });
  }

  return (
    <div className="space-y-5">
      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <Label htmlFor="ed-legend-show">Legend</Label>
          <Switch
            id="ed-legend-show"
            checked={legend.show ?? false}
            onCheckedChange={(show) => setLegend({ show })}
            aria-label="Show legend"
          />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="ed-legend-place">Placement</Label>
          <Select
            value={legend.placement ?? "top"}
            onValueChange={(v) => setLegend({ placement: v as LegendOptions["placement"] })}
          >
            <SelectTrigger id="ed-legend-place">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="top">Top</SelectItem>
              <SelectItem value="right">Right</SelectItem>
              <SelectItem value="bottom">Bottom</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </section>

      <section className="space-y-3">
        <Label>Y-axis</Label>
        <div className="space-y-1.5">
          <Label htmlFor="ed-axis-scale" className="text-xs text-muted-foreground">
            Scale
          </Label>
          <Select
            value={yAxis.scale ?? "linear"}
            onValueChange={(v) => setAxis({ scale: v as AxisOptions["scale"] })}
          >
            <SelectTrigger id="ed-axis-scale">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="linear">Linear</SelectItem>
              <SelectItem value="log">Logarithmic</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1.5">
            <Label htmlFor="ed-axis-min" className="text-xs text-muted-foreground">
              Soft min
            </Label>
            <Input
              id="ed-axis-min"
              type="number"
              value={yAxis.softMin ?? ""}
              onChange={(e) =>
                setAxis({ softMin: e.target.value === "" ? undefined : Number(e.target.value) })
              }
              placeholder="auto"
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="ed-axis-max" className="text-xs text-muted-foreground">
              Soft max
            </Label>
            <Input
              id="ed-axis-max"
              type="number"
              value={yAxis.softMax ?? ""}
              onChange={(e) =>
                setAxis({ softMax: e.target.value === "" ? undefined : Number(e.target.value) })
              }
              placeholder="auto"
            />
          </div>
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="ed-axis-label" className="text-xs text-muted-foreground">
            Label
          </Label>
          <Input
            id="ed-axis-label"
            value={yAxis.label ?? ""}
            onChange={(e) => setAxis({ label: e.target.value || undefined })}
            placeholder="e.g. kW"
          />
        </div>
      </section>
    </div>
  );
}
