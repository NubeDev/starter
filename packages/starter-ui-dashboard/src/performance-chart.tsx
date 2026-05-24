import { motion } from "motion/react";
import { cn } from "@nube/starter-ui-kit/lib/utils";

export interface PerformanceChartProps {
  /** Series values (one per labeled tick). At least 2 points needed
   * for a meaningful curve. */
  data: number[];
  /** X-axis tick labels. Render in the same order as `data`. */
  labels: string[];
  /** Section heading (already localized). */
  title: string;
  /** Optional headline value rendered next to the title (e.g. "42.3"). */
  headline?: string;
  /** Optional small suffix after the headline (e.g. "kWh"). */
  headlineSuffix?: string;
  /** Optional delta caption (e.g. "↑ 12.4%"). */
  delta?: string;
  /** Optional period selector items (e.g. `["1D","1W","1M","1Y"]`). */
  periods?: string[];
  /** 0-based index of the active period; ignored if `periods` is empty. */
  activePeriodIndex?: number;
  /** Called when a period is clicked. */
  onPeriodChange?: (index: number) => void;
  /** Stroke / accent color for the line. Default `hsl(var(--primary))`. */
  accent?: string;
  /** Extra classes merged onto the card root. */
  className?: string;
}

/** Smoothed line chart with area fill and grid lines. Theme-agnostic
 * — uses shadcn tokens; pass `accent` to colour the stroke. Periods
 * are optional and fire `onPeriodChange` on click. */
export function PerformanceChart({
  data,
  labels,
  title,
  headline,
  headlineSuffix,
  delta,
  periods,
  activePeriodIndex = 0,
  onPeriodChange,
  accent = "hsl(var(--primary))",
  className,
}: PerformanceChartProps) {
  const w = 720;
  const h = 240;
  const padX = 24;
  const padY = 24;
  const max = (data.length ? Math.max(...data) : 1) * 1.1;
  const min = 0;
  const range = max - min || 1;

  const points = data.map((v, i) => {
    const x = padX + (data.length > 1 ? (i / (data.length - 1)) * (w - padX * 2) : 0);
    const y = h - padY - ((v - min) / range) * (h - padY * 2);
    return [x, y] as const;
  });

  // Smooth path via cubic bezier
  const path = points.reduce((acc, point, i) => {
    const [x, y] = point;
    if (i === 0) return `M ${x} ${y}`;
    const prev = points[i - 1];
    if (!prev) return acc;
    const [px, py] = prev;
    const cx = (px + x) / 2;
    return `${acc} C ${cx} ${py}, ${cx} ${y}, ${x} ${y}`;
  }, "");

  const first = points[0];
  const last = points[points.length - 1];
  const area = first && last ? `${path} L ${last[0]} ${h - padY} L ${first[0]} ${h - padY} Z` : "";

  return (
    <div
      className={cn(
        "bg-card text-card-foreground border-border relative overflow-hidden rounded-3xl border p-6",
        className,
      )}
    >
      <div className="mb-4 flex items-start justify-between">
        <div>
          <div className="text-muted-foreground text-[11px] font-medium uppercase tracking-[0.18em]">
            {title}
          </div>
          {(headline || delta) && (
            <div className="mt-1 flex items-baseline gap-2">
              {headline && (
                <div className="text-3xl font-semibold tracking-tight tabular-nums">
                  {headline}
                  {headlineSuffix && (
                    <span className="text-muted-foreground text-base">{headlineSuffix}</span>
                  )}
                </div>
              )}
              {delta && <div className="text-primary text-sm">{delta}</div>}
            </div>
          )}
        </div>
        {periods && periods.length > 0 && (
          <div className="border-border bg-muted flex gap-1 rounded-full border p-1 text-[11px]">
            {periods.map((p, i) => (
              <button
                key={p}
                type="button"
                onClick={() => onPeriodChange?.(i)}
                className={cn(
                  "cursor-pointer rounded-full px-3 py-1 transition-colors",
                  i === activePeriodIndex
                    ? "bg-foreground text-background"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {p}
              </button>
            ))}
          </div>
        )}
      </div>

      <svg viewBox={`0 0 ${w} ${h}`} className="h-[240px] w-full overflow-visible">
        <defs>
          <linearGradient id="perf-area-grad" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0%" stopColor={accent} stopOpacity="0.25" />
            <stop offset="100%" stopColor={accent} stopOpacity="0" />
          </linearGradient>
        </defs>

        {/* horizontal grid */}
        {[0.25, 0.5, 0.75].map((t) => (
          <line
            key={t}
            x1={padX}
            x2={w - padX}
            y1={padY + (h - padY * 2) * t}
            y2={padY + (h - padY * 2) * t}
            className="stroke-border"
            strokeOpacity={0.4}
          />
        ))}

        {area && (
          <motion.path
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 1, delay: 0.4 }}
            d={area}
            fill="url(#perf-area-grad)"
          />
        )}
        {path && (
          <motion.path
            initial={{ pathLength: 0 }}
            animate={{ pathLength: 1 }}
            transition={{ duration: 1.6, ease: [0.22, 1, 0.36, 1] }}
            d={path}
            fill="none"
            stroke={accent}
            strokeWidth={2}
            strokeLinecap="round"
          />
        )}

        {points.map(([x, y], i) => (
          <motion.circle
            key={i}
            initial={{ opacity: 0, scale: 0 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.3, delay: 0.8 + i * 0.04 }}
            cx={x}
            cy={y}
            r={3}
            className="fill-background"
            stroke={accent}
            strokeWidth={1.5}
          />
        ))}
      </svg>

      <div className="text-muted-foreground mt-2 flex justify-between px-6 text-[10px] uppercase tracking-[0.15em]">
        {labels.map((l) => (
          <span key={l}>{l}</span>
        ))}
      </div>
    </div>
  );
}
