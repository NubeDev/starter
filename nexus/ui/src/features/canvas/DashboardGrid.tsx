import { useMemo } from "react";
import { Responsive, WidthProvider, type Layout } from "react-grid-layout";

import type { Dashboard, Widget, WidgetType } from "@/data/types";
import { PanelHost } from "@/features/widgets/PanelHost";
import { changedWidgets, toGridLayout } from "@/features/canvas/layout";
import { WIDGET_CATALOG } from "@/features/widgets/catalog";

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
  dropType,
  selectedId,
  onLayoutChange,
  onRemovePanel,
  onDropWidget,
  onSelectPanel,
  onDuplicatePanel,
}: {
  dashboard: Dashboard;
  editing: boolean;
  /** The viz type currently being dragged from the palette, if any. Sizes
   *  the drop placeholder so the ghost matches the panel that will land. */
  dropType?: WidgetType | null;
  /** The panel whose properties are open, highlighted on the canvas. */
  selectedId?: string | null;
  onLayoutChange?: (widgets: Widget[]) => void;
  onRemovePanel?: (panelId: string) => void;
  /** Fired when a palette tile is dropped on the grid, with the cell the
   *  drop landed on. The page turns this into a draft panel. */
  onDropWidget?: (position: { x: number; y: number }) => void;
  /** Open a panel's properties (edit mode). */
  onSelectPanel?: (panelId: string) => void;
  /** Duplicate a panel (edit mode). */
  onDuplicatePanel?: (panelId: string) => void;
}) {
  const layout = useMemo(
    () => toGridLayout(dashboard.widgets),
    [dashboard.widgets],
  );

  const handleChange = (current: Layout[]) => {
    if (!editing || !onLayoutChange) return;
    // Persist only the panels that actually moved — react-grid-layout
    // fires this on mount/resize too, and PATCHing unchanged panels would
    // churn the backend.
    const moved = changedWidgets(dashboard.widgets, current);
    if (moved.length > 0) onLayoutChange(moved);
  };

  // The placeholder shown while a palette tile hovers the grid — sized to
  // the dragged type's default footprint so the ghost previews the real
  // panel. `i` is the reserved key react-grid-layout uses for the drop.
  const dropSize = dropType ? WIDGET_CATALOG[dropType].defaultSize : null;
  const droppingItem = dropSize
    ? { i: "__dropping__", w: dropSize.w, h: dropSize.h }
    : undefined;

  const handleDrop = (_layout: Layout[], item: Layout) => {
    if (!editing || !onDropWidget || !dropType) return;
    onDropWidget({ x: item.x, y: item.y });
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
      isDroppable={editing}
      droppingItem={droppingItem}
      onDrop={handleDrop}
      draggableHandle=".widget-drag-handle"
      onLayoutChange={handleChange}
      useCSSTransforms
    >
      {dashboard.widgets.map((widget) => (
        <div key={widget.id}>
          <PanelHost
            widget={widget}
            editing={editing}
            selected={selectedId === widget.id}
            onRemove={
              onRemovePanel ? () => onRemovePanel(widget.id) : undefined
            }
            onSelect={
              onSelectPanel ? () => onSelectPanel(widget.id) : undefined
            }
            onDuplicate={
              onDuplicatePanel ? () => onDuplicatePanel(widget.id) : undefined
            }
          />
        </div>
      ))}
    </ResponsiveGrid>
  );
}
