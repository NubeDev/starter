import { useMemo, useState } from "react";
import { useParams } from "react-router-dom";
import { toast } from "sonner";
import {
  Check,
  Clock,
  Pencil,
  Plus,
  RotateCcw,
  Share2,
  SlidersHorizontal,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { DashboardGrid } from "@/components/DashboardGrid";
import { AddWidgetDialog } from "@/components/AddWidgetDialog";
import { DashIcon } from "@/lib/icon";
import { cn } from "@/lib/utils";
import { useDashboard } from "@/providers/useStore";
import { store } from "@/providers/store";
import { SEED_DASHBOARDS } from "@/data/seed";
import type { Widget } from "@/data/types";

const RANGES = ["15m", "1h", "6h", "24h", "7d"];

export function DashboardPage() {
  const { slug } = useParams();
  const dashboard = useDashboard(slug);
  const [editing, setEditing] = useState(false);
  const [addOpen, setAddOpen] = useState(false);
  const [range, setRange] = useState("1h");

  const nextY = useMemo(() => {
    if (!dashboard?.widgets.length) return 0;
    return Math.max(...dashboard.widgets.map((w) => w.layout.y + w.layout.h));
  }, [dashboard]);

  if (!dashboard) {
    return (
      <div className="grid h-full place-items-center text-center">
        <div>
          <div className="text-lg font-semibold">Dashboard not found</div>
          <div className="text-sm text-muted-foreground">It may have been deleted.</div>
        </div>
      </div>
    );
  }

  const setWidgets = (widgets: Widget[]) => store.setWidgets(dashboard.id, widgets);

  const onRemove = (id: string) => {
    setWidgets(dashboard.widgets.filter((w) => w.id !== id));
    toast.success("Widget removed");
  };
  const onDuplicate = (id: string) => {
    const src = dashboard.widgets.find((w) => w.id === id);
    if (!src) return;
    const copy: Widget = {
      ...src,
      id: `w${Date.now().toString(36)}`,
      title: `${src.title} copy`,
      layout: { ...src.layout, y: nextY },
      config: { ...src.config, metric: `${src.config.metric}.copy${Date.now().toString(36)}` },
    };
    setWidgets([...dashboard.widgets, copy]);
  };
  const onAdd = (w: Widget) => {
    setWidgets([...dashboard.widgets, w]);
    toast.success("Widget added");
  };
  const resetLayout = () => {
    const seed = SEED_DASHBOARDS.find((s) => s.id === dashboard.id);
    if (seed) {
      store.setWidgets(dashboard.id, structuredClone(seed.widgets));
      toast.success("Layout reset");
    }
  };

  return (
    <div className="flex h-full flex-col">
      {/* Toolbar */}
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-white/[0.06] px-6 py-4">
        <div className="flex items-center gap-3.5">
          <div
            className="grid h-11 w-11 place-items-center rounded-xl ring-1"
            style={{
              background: `hsl(${dashboard.accent} / 0.12)`,
              color: `hsl(${dashboard.accent})`,
              boxShadow: `inset 0 0 0 1px hsl(${dashboard.accent} / 0.25)`,
            }}
          >
            <DashIcon name={dashboard.icon} className="h-5 w-5" />
          </div>
          <div>
            <div className="flex items-center gap-2.5">
              <h1 className="text-xl font-bold tracking-tight text-foreground">{dashboard.name}</h1>
              <Badge variant="success" className="gap-1.5">
                <span className="relative flex h-1.5 w-1.5">
                  <span className="absolute inline-flex h-full w-full animate-pulse-ring rounded-full bg-success" />
                  <span className="relative inline-flex h-1.5 w-1.5 rounded-full bg-success" />
                </span>
                Live
              </Badge>
            </div>
            <p className="text-sm text-muted-foreground">{dashboard.description}</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* time range */}
          <div className="hidden items-center rounded-lg border border-white/8 bg-white/[0.02] p-0.5 sm:flex">
            <Clock className="mx-1.5 h-3.5 w-3.5 text-muted-foreground" />
            {RANGES.map((r) => (
              <button
                key={r}
                onClick={() => setRange(r)}
                className={cn(
                  "tabular cursor-pointer rounded-md px-2.5 py-1 text-xs font-medium transition-colors",
                  range === r
                    ? "bg-white/10 text-foreground"
                    : "text-muted-foreground hover:text-foreground"
                )}
              >
                {r}
              </button>
            ))}
          </div>

          {editing ? (
            <>
              <Button variant="outline" size="sm" onClick={resetLayout}>
                <RotateCcw /> Reset
              </Button>
              <Button variant="outline" size="sm" onClick={() => setAddOpen(true)}>
                <Plus /> Add widget
              </Button>
              <Button size="sm" onClick={() => { setEditing(false); toast.success("Layout saved"); }}>
                <Check /> Done
              </Button>
            </>
          ) : (
            <>
              <Button variant="ghost" size="icon-sm" aria-label="Share">
                <Share2 className="h-4 w-4" />
              </Button>
              <Button variant="ghost" size="icon-sm" aria-label="Settings">
                <SlidersHorizontal className="h-4 w-4" />
              </Button>
              <Button variant="outline" size="sm" onClick={() => setEditing(true)}>
                <Pencil /> Edit
              </Button>
            </>
          )}
        </div>
      </div>

      {/* Canvas */}
      <div className="scrollbar-thin flex-1 overflow-y-auto p-6">
        {editing && (
          <div className="mb-4 flex items-center gap-2 rounded-xl border border-dashed border-primary/30 bg-primary/[0.04] px-4 py-2.5 text-sm text-primary">
            <SlidersHorizontal className="h-4 w-4" />
            Edit mode — drag the handle to move, pull the corner to resize, or add new widgets.
          </div>
        )}
        <DashboardGrid
          dashboard={dashboard}
          editing={editing}
          onLayoutChange={setWidgets}
          onRemove={onRemove}
          onDuplicate={onDuplicate}
        />
      </div>

      <AddWidgetDialog open={addOpen} onOpenChange={setAddOpen} onAdd={onAdd} nextY={nextY} />
    </div>
  );
}
