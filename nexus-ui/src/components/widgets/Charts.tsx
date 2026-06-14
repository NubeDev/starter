import { useMemo } from "react";
import {
  Area,
  AreaChart,
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { series, jitter } from "@/data/fake";
import { useTick } from "@/lib/hooks";
import type { Widget } from "@/data/types";

function hsl(c?: string, fallback = "152 76% 44%") {
  return `hsl(${c ?? fallback})`;
}

function ChartTooltip({ active, payload, label, unit }: any) {
  if (!active || !payload?.length) return null;
  return (
    <div className="rounded-lg glass px-3 py-2 text-xs shadow-xl">
      <div className="mb-0.5 text-muted-foreground">{label}</div>
      <div className="tabular text-sm font-semibold text-foreground">
        {payload[0].value}
        {unit ? <span className="ml-1 text-muted-foreground">{unit}</span> : null}
      </div>
    </div>
  );
}

export function LineWidget({ widget }: { widget: Widget }) {
  const tick = useTick(3000);
  const color = hsl(widget.config.color);
  const data = useMemo(() => {
    const base = series(widget.config.metric, { points: 48, base: 56, amplitude: 22, noise: 5 });
    // gently nudge the last few points so the line feels alive
    return base.map((p, i) => (i >= base.length - 3 ? { ...p, v: jitter(p.v, 0.04) } : p));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [widget.config.metric, tick]);

  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart data={data} margin={{ top: 8, right: 10, bottom: 0, left: -18 }}>
        <CartesianGrid strokeDasharray="3 6" stroke="hsl(217 33% 100% / 0.05)" vertical={false} />
        <XAxis dataKey="t" tick={{ fill: "hsl(215 20% 55%)", fontSize: 11 }} tickLine={false} axisLine={false} minTickGap={36} />
        <YAxis tick={{ fill: "hsl(215 20% 45%)", fontSize: 11 }} tickLine={false} axisLine={false} width={42} />
        <Tooltip content={<ChartTooltip unit={widget.config.unit} />} cursor={{ stroke: color, strokeOpacity: 0.3 }} />
        <Line
          type="monotone"
          dataKey="v"
          stroke={color}
          strokeWidth={2.4}
          dot={false}
          activeDot={{ r: 4, fill: color, stroke: "hsl(222 47% 4%)", strokeWidth: 2 }}
          isAnimationActive
          animationDuration={700}
        />
      </LineChart>
    </ResponsiveContainer>
  );
}

export function AreaWidget({ widget }: { widget: Widget }) {
  const tick = useTick(3200);
  const color = hsl(widget.config.color, "199 90% 56%");
  const gid = `grad-${widget.id}`;
  const data = useMemo(() => {
    const base = series(widget.config.metric, { points: 48, base: 48, amplitude: 18, noise: 4, trend: 6 });
    return base.map((p, i) => (i >= base.length - 3 ? { ...p, v: jitter(p.v, 0.04) } : p));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [widget.config.metric, tick]);

  return (
    <ResponsiveContainer width="100%" height="100%">
      <AreaChart data={data} margin={{ top: 8, right: 10, bottom: 0, left: -18 }}>
        <defs>
          <linearGradient id={gid} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={color} stopOpacity={0.45} />
            <stop offset="100%" stopColor={color} stopOpacity={0.02} />
          </linearGradient>
        </defs>
        <CartesianGrid strokeDasharray="3 6" stroke="hsl(217 33% 100% / 0.05)" vertical={false} />
        <XAxis dataKey="t" tick={{ fill: "hsl(215 20% 55%)", fontSize: 11 }} tickLine={false} axisLine={false} minTickGap={36} />
        <YAxis tick={{ fill: "hsl(215 20% 45%)", fontSize: 11 }} tickLine={false} axisLine={false} width={42} />
        <Tooltip content={<ChartTooltip unit={widget.config.unit} />} cursor={{ stroke: color, strokeOpacity: 0.3 }} />
        <Area type="monotone" dataKey="v" stroke={color} strokeWidth={2.4} fill={`url(#${gid})`} isAnimationActive animationDuration={700} />
      </AreaChart>
    </ResponsiveContainer>
  );
}
