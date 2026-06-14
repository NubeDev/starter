import { useState } from "react";
import {
  Activity,
  AreaChart as AreaIcon,
  Gauge as GaugeIcon,
  LineChart as LineIcon,
  ListChecks,
  Table as TableIcon,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { Widget, WidgetType } from "@/data/types";

const ACCENTS = ["152 76% 44%", "199 90% 56%", "263 80% 66%", "38 95% 56%", "346 84% 60%"];

const TYPES: {
  type: WidgetType;
  label: string;
  desc: string;
  icon: typeof Activity;
  size: { w: number; h: number };
}[] = [
  { type: "line", label: "Line Chart", desc: "Trend over time", icon: LineIcon, size: { w: 6, h: 4 } },
  { type: "area", label: "Area Chart", desc: "Volume / cumulative", icon: AreaIcon, size: { w: 6, h: 4 } },
  { type: "gauge", label: "Gauge", desc: "Value vs threshold", icon: GaugeIcon, size: { w: 3, h: 4 } },
  { type: "stat", label: "Stat / KPI", desc: "Single metric + spark", icon: Activity, size: { w: 3, h: 2 } },
  { type: "status", label: "Status List", desc: "Subsystem health", icon: ListChecks, size: { w: 4, h: 4 } },
  { type: "table", label: "Device Table", desc: "Tabular telemetry", icon: TableIcon, size: { w: 6, h: 5 } },
];

interface Props {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  onAdd: (w: Widget) => void;
  nextY: number;
}

export function AddWidgetDialog({ open, onOpenChange, onAdd, nextY }: Props) {
  const [type, setType] = useState<WidgetType>("line");
  const [title, setTitle] = useState("");
  const [metric, setMetric] = useState("");
  const [accent, setAccent] = useState(ACCENTS[0]);

  const selected = TYPES.find((t) => t.type === type)!;

  const submit = () => {
    const id = `w${Date.now().toString(36)}`;
    const t = title.trim() || selected.label;
    const w: Widget = {
      id,
      type,
      title: t,
      config: {
        metric: (metric.trim() || t.toLowerCase().replace(/\s+/g, ".")) + `.${id}`,
        color: accent,
        unit: type === "gauge" ? "%" : "",
        min: 0,
        max: 100,
        warn: 70,
        crit: 90,
        decimals: type === "stat" ? 1 : 0,
      },
      layout: { x: 0, y: nextY, w: selected.size.w, h: selected.size.h },
    };
    onAdd(w);
    setTitle("");
    setMetric("");
    setType("line");
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>Add a widget</DialogTitle>
          <DialogDescription>Pick a visualization and drop it onto the canvas.</DialogDescription>
        </DialogHeader>

        <div className="grid grid-cols-3 gap-2.5">
          {TYPES.map((t) => {
            const Icon = t.icon;
            const active = t.type === type;
            return (
              <button
                key={t.type}
                onClick={() => setType(t.type)}
                className={cn(
                  "group flex cursor-pointer flex-col items-start gap-2 rounded-xl border p-3 text-left transition-all",
                  active
                    ? "border-primary/50 bg-primary/10 ring-glow"
                    : "border-white/8 bg-white/[0.02] hover:border-white/20 hover:bg-white/[0.04]"
                )}
              >
                <Icon className={cn("h-5 w-5", active ? "text-primary" : "text-muted-foreground")} />
                <div>
                  <div className="text-sm font-medium text-foreground">{t.label}</div>
                  <div className="text-[0.7rem] text-muted-foreground">{t.desc}</div>
                </div>
              </button>
            );
          })}
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <div className="space-y-1.5">
            <Label htmlFor="w-title">Title</Label>
            <Input id="w-title" value={title} onChange={(e) => setTitle(e.target.value)} placeholder={selected.label} />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="w-metric">Metric key</Label>
            <Input id="w-metric" value={metric} onChange={(e) => setMetric(e.target.value)} placeholder="e.g. sensor.temp" />
          </div>
        </div>

        <div className="space-y-2">
          <Label>Accent</Label>
          <div className="flex items-center gap-2.5">
            {ACCENTS.map((a) => (
              <button
                key={a}
                onClick={() => setAccent(a)}
                aria-label={`accent ${a}`}
                className={cn(
                  "h-7 w-7 cursor-pointer rounded-full transition-transform hover:scale-110",
                  accent === a && "ring-2 ring-white/80 ring-offset-2 ring-offset-background"
                )}
                style={{ background: `hsl(${a})` }}
              />
            ))}
          </div>
        </div>

        <div className="mt-1 flex justify-end gap-2">
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={submit}>Add widget</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
