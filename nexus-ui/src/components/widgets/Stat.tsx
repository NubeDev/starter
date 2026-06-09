import { useMemo } from "react";
import { Area, AreaChart, ResponsiveContainer } from "recharts";
import { ArrowDownRight, ArrowUpRight, Minus } from "lucide-react";
import { series, statValue, jitter } from "@/data/fake";
import { useTick, useAnimatedNumber } from "@/lib/hooks";
import { cn } from "@/lib/utils";
import type { Widget } from "@/data/types";

const BASES: Record<string, [number, number]> = {
  "fleet.online": [1284, 40],
  "fleet.alerts": [7, 6],
  "fleet.uptime": [99.92, 0.2],
};

export function StatWidget({ widget }: { widget: Widget }) {
  const tick = useTick(2800);
  const color = widget.config.color ?? "152 76% 44%";
  const decimals = widget.config.decimals ?? 1;

  const [base, spread] = BASES[widget.config.metric] ?? [
    deriveBase(widget.config.metric),
    deriveBase(widget.config.metric) * 0.3,
  ];

  const { value, deltaPct } = useMemo(() => {
    const s = statValue(widget.config.metric, base, spread);
    return { value: jitter(s.value, 0.01), deltaPct: s.deltaPct };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [widget.config.metric, tick]);

  const spark = useMemo(
    () => series(widget.config.metric + ":spark", { points: 24, base: 50, amplitude: 16, noise: 4 }),
    [widget.config.metric]
  );

  const animated = useAnimatedNumber(value, 600);
  const trend = deltaPct > 0.5 ? "up" : deltaPct < -0.5 ? "down" : "flat";
  const TrendIcon = trend === "up" ? ArrowUpRight : trend === "down" ? ArrowDownRight : Minus;
  const trendColor =
    trend === "up" ? "text-success" : trend === "down" ? "text-destructive" : "text-muted-foreground";
  const gid = `spark-${widget.id}`;

  return (
    <div className="flex h-full w-full flex-col justify-between">
      <div className="flex items-start justify-between">
        <div className="min-w-0">
          <div className="tabular text-[2rem] font-bold leading-tight tracking-tight text-foreground">
            {animated.toFixed(decimals)}
            {widget.config.unit ? (
              <span className="ml-1 text-base font-medium text-muted-foreground">
                {widget.config.unit}
              </span>
            ) : null}
          </div>
          <div className={cn("mt-1 flex items-center gap-1 text-xs font-medium", trendColor)}>
            <TrendIcon className="h-3.5 w-3.5" />
            <span className="tabular">{Math.abs(deltaPct).toFixed(1)}%</span>
            <span className="text-muted-foreground">vs last hour</span>
          </div>
        </div>
      </div>
      <div className="-mx-1 -mb-1 h-10">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={spark} margin={{ top: 4, right: 0, bottom: 0, left: 0 }}>
            <defs>
              <linearGradient id={gid} x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor={`hsl(${color})`} stopOpacity={0.4} />
                <stop offset="100%" stopColor={`hsl(${color})`} stopOpacity={0} />
              </linearGradient>
            </defs>
            <Area type="monotone" dataKey="v" stroke={`hsl(${color})`} strokeWidth={2} fill={`url(#${gid})`} isAnimationActive={false} />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

function deriveBase(metric: string): number {
  let h = 0;
  for (let i = 0; i < metric.length; i++) h = (h * 31 + metric.charCodeAt(i)) % 997;
  return 20 + (h % 80);
}
