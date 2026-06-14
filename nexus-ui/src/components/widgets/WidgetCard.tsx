import { GripVertical, MoreVertical, Trash2, Copy } from "lucide-react";
import { Card } from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import type { Widget } from "@/data/types";
import { LineWidget, AreaWidget } from "./Charts";
import { GaugeWidget } from "./Gauge";
import { StatWidget } from "./Stat";
import { StatusWidget } from "./Status";
import { DeviceTableWidget } from "./DeviceTable";

function WidgetBody({ widget }: { widget: Widget }) {
  switch (widget.type) {
    case "line":
      return <LineWidget widget={widget} />;
    case "area":
      return <AreaWidget widget={widget} />;
    case "gauge":
      return <GaugeWidget widget={widget} />;
    case "stat":
      return <StatWidget widget={widget} />;
    case "status":
      return <StatusWidget widget={widget} />;
    case "table":
      return <DeviceTableWidget widget={widget} />;
    default:
      return null;
  }
}

interface Props {
  widget: Widget;
  editing: boolean;
  onRemove: (id: string) => void;
  onDuplicate: (id: string) => void;
}

export function WidgetCard({ widget, editing, onRemove, onDuplicate }: Props) {
  const accent = widget.config.color ?? "152 76% 44%";
  return (
    <Card
      className={cn(
        "group/widget card-hover relative flex h-full w-full flex-col overflow-hidden",
        editing && "ring-1 ring-white/10"
      )}
    >
      {/* accent top-line */}
      <span
        className="pointer-events-none absolute inset-x-0 top-0 h-px opacity-70"
        style={{ background: `linear-gradient(90deg, transparent, hsl(${accent} / 0.8), transparent)` }}
      />
      <div className="flex items-center justify-between gap-2 px-4 pt-3.5 pb-2">
        <div
          className={cn(
            "flex min-w-0 items-center gap-2",
            editing && "widget-drag-handle cursor-grab active:cursor-grabbing"
          )}
        >
          {editing && (
            <span className="-ml-1 rounded p-0.5 text-muted-foreground/60">
              <GripVertical className="h-4 w-4" />
            </span>
          )}
          <div className="min-w-0">
            <div className="truncate text-sm font-semibold text-foreground">{widget.title}</div>
            {widget.subtitle ? (
              <div className="truncate text-xs text-muted-foreground">{widget.subtitle}</div>
            ) : null}
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          <span
            className="hidden h-1.5 w-1.5 rounded-full sm:inline-block"
            style={{ background: `hsl(${accent})`, boxShadow: `0 0 8px hsl(${accent} / 0.8)` }}
          />
          {editing && (
            <DropdownMenu>
              <DropdownMenuTrigger className="rounded-md p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-white/5 hover:text-foreground focus:outline-none group-hover/widget:opacity-100 cursor-pointer">
                <MoreVertical className="h-4 w-4" />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => onDuplicate(widget.id)}>
                  <Copy /> Duplicate
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onClick={() => onRemove(widget.id)}
                  className="text-destructive focus:text-destructive [&>svg]:text-destructive"
                >
                  <Trash2 /> Remove
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          )}
        </div>
      </div>
      <div className="min-h-0 flex-1 px-4 pb-4">
        <WidgetBody widget={widget} />
      </div>
    </Card>
  );
}
