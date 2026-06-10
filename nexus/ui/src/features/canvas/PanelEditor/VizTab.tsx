import { Plus, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import type { SeriesField, WidgetType } from "@/data/types";
import type { EditorDraft } from "@/features/canvas/PanelEditor/useEditorDraft";
import { WIDGET_CATALOG, WIDGET_TYPES } from "@/features/widgets/catalog";

// Visualization tab: pick the panel type and map result columns onto
// chart roles. Unlike the legacy side panel (which edited only the first
// series' value), this manages the full series list — add, remove, rename,
// and map each series to a column — plus the x column when the type needs
// one. Per-series colour/unit overrides live in the Overrides tab.
export function VizTab({ draft }: { draft: EditorDraft }) {
  const { widget, patch, patchConfig } = draft;
  const { fields } = widget.config;
  const needsX = WIDGET_CATALOG[widget.type].roles.x === "required";
  const multi = WIDGET_CATALOG[widget.type].roles.series === "multi";

  function setSeries(series: SeriesField[]) {
    patchConfig({ fields: { ...fields, series } });
  }

  return (
    <div className="space-y-4">
      <div className="space-y-1.5">
        <Label htmlFor="ed-type">Visualization</Label>
        <Select value={widget.type} onValueChange={(v) => patch({ type: v as WidgetType })}>
          <SelectTrigger id="ed-type">
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

      {needsX ? (
        <div className="space-y-1.5">
          <Label htmlFor="ed-x">X column</Label>
          <Input
            id="ed-x"
            value={fields.x ?? ""}
            onChange={(e) =>
              patchConfig({ fields: { ...fields, x: e.target.value || undefined } })
            }
            placeholder="ts"
          />
        </div>
      ) : null}

      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label>Series</Label>
          {multi ? (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-7 gap-1 px-2 text-xs"
              onClick={() => setSeries([...fields.series, { value: "" }])}
            >
              <Plus className="size-3.5" /> Add series
            </Button>
          ) : null}
        </div>

        {fields.series.length === 0 ? (
          <p className="text-xs text-muted-foreground">
            No series yet — add one and map it to a result column.
          </p>
        ) : null}

        {fields.series.map((s, i) => (
          <div key={i} className="flex items-end gap-2">
            <div className="flex-1 space-y-1">
              <Label htmlFor={`ed-series-val-${i}`} className="text-xs text-muted-foreground">
                Column
              </Label>
              <Input
                id={`ed-series-val-${i}`}
                value={s.value}
                onChange={(e) =>
                  setSeries(fields.series.map((x, j) => (j === i ? { ...x, value: e.target.value } : x)))
                }
                placeholder="value"
              />
            </div>
            <div className="flex-1 space-y-1">
              <Label htmlFor={`ed-series-lbl-${i}`} className="text-xs text-muted-foreground">
                Label
              </Label>
              <Input
                id={`ed-series-lbl-${i}`}
                value={s.label ?? ""}
                onChange={(e) =>
                  setSeries(
                    fields.series.map((x, j) =>
                      j === i ? { ...x, label: e.target.value || undefined } : x,
                    ),
                  )
                }
                placeholder={s.value}
              />
            </div>
            {/* The first series can't be removed for single-series panels;
                multi panels can drop any series. */}
            {multi || i > 0 ? (
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label={`Remove series ${i + 1}`}
                className="size-9 text-muted-foreground hover:text-destructive"
                onClick={() => setSeries(fields.series.filter((_, j) => j !== i))}
              >
                <Trash2 className="size-4" />
              </Button>
            ) : null}
          </div>
        ))}
      </div>
    </div>
  );
}
