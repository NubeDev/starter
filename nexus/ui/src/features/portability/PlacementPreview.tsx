import { useMemo } from "react";
import { Check } from "lucide-react";

import type { PanelExport } from "@/api/types";
import { WIDGET_CATALOG } from "@/features/widgets/catalog";
import { widgetIcon } from "@/features/widgets/icon";
import {
  exportWidgetType,
  readExportLayout,
  type PortableSelection,
} from "@/features/portability/model";

// The grid width the live canvas uses at desktop (DashboardGrid `cols.lg`).
// The preview mirrors it so exported panels sit exactly where they live.
const GRID_COLS = 12;

// A schematic, data-free preview of a dashboard export's panels laid out on
// their real grid. Each tile shows the panel's viz icon, title, and footprint —
// not a live chart, because an exported model may reference a datasource this
// tenant can't query, and a fake chart would be dishonest (F0). Tiles are
// click-to-toggle: a deselected panel dims and un-checks but stays visible so
// the user keeps spatial context while choosing what to include.
export function PlacementPreview({
  panels,
  selection,
  onToggle,
  selectable = true,
}: {
  panels: ReadonlyArray<PanelExport>;
  selection: PortableSelection;
  onToggle?: (index: number) => void;
  /** When false, tiles render read-only (no checkbox, no dimming) — used for a
   *  pure preview with selection handled elsewhere. */
  selectable?: boolean;
}) {
  // The number of grid rows the tallest panel reaches, so the container is tall
  // enough to show every tile (react-grid-layout grows unbounded; here we size
  // to content). Minimum a few rows so an empty/short board still has presence.
  const rows = useMemo(() => {
    const max = panels.reduce((acc, p) => {
      const l = readExportLayout(p.layout);
      return Math.max(acc, l.y + l.h);
    }, 0);
    return Math.max(max, 4);
  }, [panels]);

  if (panels.length === 0) {
    return (
      <div className="flex h-40 items-center justify-center rounded-xl border border-dashed border-border text-sm text-muted-foreground">
        No widgets to preview.
      </div>
    );
  }

  return (
    <div
      className="grid gap-2 rounded-xl bg-muted/30 p-2"
      style={{
        gridTemplateColumns: `repeat(${GRID_COLS}, minmax(0, 1fr))`,
        gridAutoRows: "2rem",
      }}
    >
      {panels.map((panel, index) => {
        const layout = readExportLayout(panel.layout);
        const type = exportWidgetType(panel);
        const Icon = widgetIcon(WIDGET_CATALOG[type].icon);
        const selected = selection.panelIndices.has(index);
        // Clamp a panel to the grid so a malformed export can't push a tile
        // off-canvas (col-start is 1-based).
        const colStart = Math.min(Math.max(layout.x, 0), GRID_COLS - 1) + 1;
        const colSpan = Math.min(layout.w, GRID_COLS - (colStart - 1));
        return (
          <button
            key={index}
            type="button"
            disabled={!selectable}
            onClick={() => onToggle?.(index)}
            style={{
              gridColumn: `${colStart} / span ${Math.max(colSpan, 1)}`,
              gridRow: `${layout.y + 1} / span ${Math.max(layout.h, 1)}`,
            }}
            className={[
              "group relative flex flex-col items-start gap-1 overflow-hidden rounded-lg border p-2 text-left transition",
              selectable ? "cursor-pointer" : "cursor-default",
              selected || !selectable
                ? "border-primary/40 bg-card shadow-sm"
                : "border-border bg-card/40 opacity-50 hover:opacity-80",
            ].join(" ")}
            title={panel.title || "Untitled panel"}
          >
            {selectable ? (
              <span
                className={[
                  "absolute right-1.5 top-1.5 flex size-4 items-center justify-center rounded border",
                  selected
                    ? "border-primary bg-primary text-primary-foreground"
                    : "border-border bg-background",
                ].join(" ")}
                aria-hidden
              >
                {selected ? <Check className="size-3" /> : null}
              </span>
            ) : null}
            <Icon className="size-4 shrink-0 text-muted-foreground" />
            <span className="line-clamp-2 text-xs font-medium leading-tight text-foreground">
              {panel.title || "Untitled panel"}
            </span>
            <span className="mt-auto text-[10px] uppercase tracking-wide text-muted-foreground">
              {WIDGET_CATALOG[type].label}
            </span>
          </button>
        );
      })}
      {/* A spacer cell to ensure the implicit grid is at least `rows` tall even
          if the tallest panel is short, so the board's proportions read right. */}
      <span
        aria-hidden
        style={{ gridColumn: "1 / 2", gridRow: `${rows} / span 1` }}
      />
    </div>
  );
}
