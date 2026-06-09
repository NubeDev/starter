import { useMemo } from "react";
import { devices } from "@/data/fake";
import { Badge } from "@/components/ui/badge";
import type { Widget } from "@/data/types";

function Signal({ value }: { value: number }) {
  const bars = Math.round((value / 100) * 4);
  return (
    <div className="flex items-end gap-0.5" title={`${value}%`}>
      {[1, 2, 3, 4].map((b) => (
        <span
          key={b}
          className="w-1 rounded-sm"
          style={{
            height: `${b * 3 + 2}px`,
            background: b <= bars ? "hsl(152 76% 44%)" : "hsl(217 33% 22%)",
          }}
        />
      ))}
    </div>
  );
}

function Battery({ value }: { value: number }) {
  const color = value < 20 ? "0 72% 55%" : value < 50 ? "38 95% 56%" : "152 76% 44%";
  return (
    <div className="flex items-center gap-2">
      <div className="relative h-3 w-7 rounded-[3px] border border-white/20">
        <div className="absolute inset-y-0.5 left-0.5 rounded-[1px]" style={{ width: `${value * 0.22}px`, background: `hsl(${color})` }} />
      </div>
      <span className="tabular text-xs text-muted-foreground">{value}%</span>
    </div>
  );
}

export function DeviceTableWidget({ widget }: { widget: Widget }) {
  const rows = useMemo(() => devices(widget.config.metric, 9), [widget.config.metric]);

  return (
    <div className="scrollbar-thin h-full w-full overflow-auto">
      <table className="w-full border-collapse text-sm">
        <thead className="sticky top-0 z-10">
          <tr className="bg-card/80 text-left text-[0.7rem] uppercase tracking-wider text-muted-foreground backdrop-blur">
            <th className="px-3 py-2 font-medium">Device</th>
            <th className="px-3 py-2 font-medium">Site</th>
            <th className="px-3 py-2 font-medium">Status</th>
            <th className="px-3 py-2 font-medium">Signal</th>
            <th className="px-3 py-2 font-medium">Battery</th>
            <th className="px-3 py-2 text-right font-medium">Last seen</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((d) => (
            <tr key={d.id} className="border-t border-white/5 transition-colors hover:bg-white/[0.03]">
              <td className="px-3 py-2.5">
                <div className="font-medium text-foreground">{d.name}</div>
                <div className="tabular text-[0.7rem] text-muted-foreground">{d.id}</div>
              </td>
              <td className="px-3 py-2.5 text-muted-foreground">{d.site}</td>
              <td className="px-3 py-2.5">
                <Badge
                  variant={d.status === "online" ? "success" : d.status === "degraded" ? "warning" : "danger"}
                >
                  {d.status}
                </Badge>
              </td>
              <td className="px-3 py-2.5"><Signal value={d.signal} /></td>
              <td className="px-3 py-2.5"><Battery value={d.battery} /></td>
              <td className="px-3 py-2.5 text-right tabular text-xs text-muted-foreground">{d.lastSeen}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
