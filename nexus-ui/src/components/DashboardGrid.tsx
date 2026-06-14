import { useMemo } from "react";
import { Responsive, WidthProvider, type Layout } from "react-grid-layout";
import { WidgetCard } from "./widgets/WidgetCard";
import type { Dashboard, Widget } from "@/data/types";

const ResponsiveGrid = WidthProvider(Responsive);

const MIN_SIZES: Record<Widget["type"], { minW: number; minH: number }> = {
  stat: { minW: 2, minH: 2 },
  gauge: { minW: 2, minH: 3 },
  line: { minW: 3, minH: 3 },
  area: { minW: 3, minH: 3 },
  status: { minW: 3, minH: 3 },
  table: { minW: 4, minH: 4 },
};

interface Props {
  dashboard: Dashboard;
  editing: boolean;
  onLayoutChange: (widgets: Widget[]) => void;
  onRemove: (id: string) => void;
  onDuplicate: (id: string) => void;
}

export function DashboardGrid({ dashboard, editing, onLayoutChange, onRemove, onDuplicate }: Props) {
  const layout: Layout[] = useMemo(
    () =>
      dashboard.widgets.map((w) => ({
        i: w.id,
        x: w.layout.x,
        y: w.layout.y,
        w: w.layout.w,
        h: w.layout.h,
        ...MIN_SIZES[w.type],
      })),
    [dashboard.widgets]
  );

  const handleChange = (current: Layout[]) => {
    if (!editing) return;
    const byId = new Map(current.map((l) => [l.i, l]));
    const next = dashboard.widgets.map((w) => {
      const l = byId.get(w.id);
      return l ? { ...w, layout: { x: l.x, y: l.y, w: l.w, h: l.h } } : w;
    });
    // only persist if something actually moved
    const changed = next.some(
      (w, i) =>
        w.layout.x !== dashboard.widgets[i].layout.x ||
        w.layout.y !== dashboard.widgets[i].layout.y ||
        w.layout.w !== dashboard.widgets[i].layout.w ||
        w.layout.h !== dashboard.widgets[i].layout.h
    );
    if (changed) onLayoutChange(next);
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
      {dashboard.widgets.map((w) => (
        <div key={w.id} className="animate-widget-in">
          <WidgetCard widget={w} editing={editing} onRemove={onRemove} onDuplicate={onDuplicate} />
        </div>
      ))}
    </ResponsiveGrid>
  );
}
