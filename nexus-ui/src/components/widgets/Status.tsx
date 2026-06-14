import { useMemo } from "react";
import { devices } from "@/data/fake";
import { cn } from "@/lib/utils";
import type { Widget } from "@/data/types";

const META = {
  online: { label: "Operational", dot: "152 76% 44%", text: "text-success" },
  degraded: { label: "Degraded", dot: "38 95% 56%", text: "text-warning" },
  offline: { label: "Offline", dot: "0 72% 55%", text: "text-destructive" },
} as const;

export function StatusWidget({ widget }: { widget: Widget }) {
  const rows = useMemo(() => devices(widget.config.metric, 6), [widget.config.metric]);

  return (
    <div className="scrollbar-thin flex h-full w-full flex-col gap-2 overflow-y-auto pr-1">
      {rows.map((d) => {
        const m = META[d.status];
        return (
          <div
            key={d.id}
            className="flex items-center justify-between rounded-lg border border-white/5 bg-white/[0.02] px-3 py-2.5 transition-colors hover:bg-white/[0.04]"
          >
            <div className="flex items-center gap-3">
              <span className="relative flex h-2.5 w-2.5">
                {d.status === "online" && (
                  <span
                    className="absolute inline-flex h-full w-full animate-pulse-ring rounded-full"
                    style={{ background: `hsl(${m.dot})` }}
                  />
                )}
                <span className="relative inline-flex h-2.5 w-2.5 rounded-full" style={{ background: `hsl(${m.dot})` }} />
              </span>
              <div>
                <div className="text-sm font-medium text-foreground">{d.name}</div>
                <div className="text-xs text-muted-foreground">{d.site}</div>
              </div>
            </div>
            <div className="text-right">
              <div className={cn("text-xs font-semibold", m.text)}>{m.label}</div>
              <div className="tabular text-[0.7rem] text-muted-foreground">{d.lastSeen}</div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
