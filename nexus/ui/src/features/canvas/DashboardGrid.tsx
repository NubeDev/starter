import { useMemo } from "react";
import { Responsive, WidthProvider, type Layout } from "react-grid-layout";

import type { Dashboard, Widget } from "@/data/types";
import { PanelHost } from "@/features/widgets/PanelHost";
import { applyGridLayout, toGridLayout } from "@/features/canvas/layout";

// react-grid-layout's positioning + resize-handle chrome is provided by
// the ported styles in `index.css` (the `.react-grid-*` block), so the
// library's own stylesheets aren't imported here.
const ResponsiveGrid = WidthProvider(Responsive);

// The dashboard canvas: a responsive react-grid-layout of live panels. In
// edit mode panels drag (by their card header) and resize; the changed
// layout is handed up via `onLayoutChange` for the caller to persist.
// View mode locks the grid. Each cell mounts a `PanelHost`, which owns the
// panel's data subscription — the grid only places panels, it never
// fetches (F6).
export function DashboardGrid({
  dashboard,
  editing,
  onLayoutChange,
  onRemovePanel,
}: {
  dashboard: Dashboard;
  editing: boolean;
  onLayoutChange?: (widgets: Widget[]) => void;
  onRemovePanel?: (panelId: string) => void;
}) {
  const layout = useMemo(
    () => toGridLayout(dashboard.widgets),
    [dashboard.widgets],
  );

  const handleChange = (current: Layout[]) => {
    if (!editing || !onLayoutChange) return;
    const next = applyGridLayout(dashboard.widgets, current);
    if (next) onLayoutChange(next);
  };

  return (
    <ResponsiveGrid
      className="layout"
      layouts={{ lg: layout, md: layout }}
      breakpoints={{ lg: 1200, md: 900, sm: 640, xs: 480, xxs: 0 }}
      cols={{ lg: 12, md: 12, sm: 6, xs: 4, xxs: 2 }}
      rowHeight={62}
      margin={[16, 16]}
      containerPadding={[0, 0]}
      isDraggable={editing}
      isResizable={editing}
      draggableHandle=".widget-drag-handle"
      onLayoutChange={handleChange}
      useCSSTransforms
    >
      {dashboard.widgets.map((widget) => (
        <div key={widget.id}>
          <PanelHost
            widget={widget}
            editing={editing}
            onRemove={
              onRemovePanel ? () => onRemovePanel(widget.id) : undefined
            }
          />
        </div>
      ))}
    </ResponsiveGrid>
  );
}
