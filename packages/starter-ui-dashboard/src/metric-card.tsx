import { motion, useMotionValue, useSpring, useTransform } from "motion/react";
import { useEffect } from "react";
import { cn } from "@nube/starter-ui-kit/lib/utils";

export interface MetricCardProps {
  /** Label rendered in the top-left (already localized). */
  label: string;
  /** Numeric value; animated on mount and on change. */
  value: number;
  /** Optional preformatted display string. When set, it is shown verbatim
   *  instead of the animated number — for a value that has been run through a
   *  formatter (fixed decimals, unit, or a value-mapping that replaces the text
   *  outright, e.g. `1` → "On"). `prefix`/`suffix` are ignored when `display`
   *  is set, since the formatter already produced the full text. */
  display?: string;
  /** Optional CSS color for the value text (e.g. a value-mapping's colour). */
  valueColor?: string;
  /** Optional suffix appended after the animated number (e.g. "kWh"). */
  suffix?: string;
  /** Optional prefix prepended before the animated number (e.g. "$"). */
  prefix?: string;
  /** Percentage delta shown as a pill (e.g. `12.4` → "↑ 12.4%"). */
  delta?: number;
  /** Optional sparkline data; rendered as an inline area chart. */
  spark?: number[];
  /** Accent CSS color for the sparkline stroke. Defaults to `currentColor`
   * so the consumer's theme drives the appearance. Pass any CSS color:
   * `"#4ade80"`, `"hsl(var(--primary))"`, etc. */
  accent?: string;
  /** Extra classes merged onto the card root. */
  className?: string;
}

function useAnimatedNumber(target: number) {
  const mv = useMotionValue(0);
  const spring = useSpring(mv, { stiffness: 80, damping: 20 });
  const rounded = useTransform(spring, (v) => Math.round(v).toLocaleString());
  useEffect(() => {
    mv.set(target);
  }, [target, mv]);
  return rounded;
}

function Spark({ data, color }: { data: number[]; color: string }) {
  if (!data.length) return null;
  const max = Math.max(...data);
  const min = Math.min(...data);
  const range = max - min || 1;
  const w = 120;
  const h = 36;
  const points = data
    .map((v, i) => {
      const x = (i / (data.length - 1)) * w;
      const y = h - ((v - min) / range) * h;
      return `${x},${y}`;
    })
    .join(" ");
  // Derive a stable-but-unique id from the color so multiple cards on the
  // same page don't share the same <linearGradient>.
  const gradId = `spark-grad-${color.replace(/[^a-z0-9]/gi, "")}`;
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} className="overflow-visible">
      <defs>
        <linearGradient id={gradId} x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity="0.4" />
          <stop offset="100%" stopColor={color} stopOpacity="0" />
        </linearGradient>
      </defs>
      <motion.polyline
        initial={{ pathLength: 0, opacity: 0 }}
        animate={{ pathLength: 1, opacity: 1 }}
        transition={{ duration: 1.6, ease: [0.22, 1, 0.36, 1] }}
        fill="none"
        stroke={color}
        strokeWidth={1.75}
        strokeLinecap="round"
        strokeLinejoin="round"
        points={points}
      />
      <polygon points={`0,${h} ${points} ${w},${h}`} fill={`url(#${gradId})`} />
    </svg>
  );
}

/** Animated metric tile with a sparkline and optional delta badge.
 * Theme-agnostic — uses shadcn `--card`, `--muted-foreground`,
 * `--destructive` tokens. Pass `accent` to colour the sparkline. */
export function MetricCard({
  label,
  value,
  display,
  valueColor,
  suffix,
  prefix,
  delta,
  spark = [],
  accent = "currentColor",
  className,
}: MetricCardProps) {
  const animated = useAnimatedNumber(value);
  const deltaPositive = (delta ?? 0) >= 0;

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-50px" }}
      transition={{ duration: 0.6, ease: [0.22, 1, 0.36, 1] }}
      whileHover={{ y: -2 }}
      className={cn(
        "group bg-card text-card-foreground border-border relative flex flex-col gap-4 overflow-hidden rounded-3xl border p-6",
        className,
      )}
    >
      <div className="flex items-start justify-between">
        <div className="text-muted-foreground text-[11px] font-medium uppercase tracking-[0.18em]">
          {label}
        </div>
        {typeof delta === "number" && (
          <div
            className={cn(
              "rounded-full px-2 py-0.5 text-[10px] font-semibold tabular-nums",
              deltaPositive
                ? "bg-emerald-500/15 text-emerald-500"
                : "bg-destructive/15 text-destructive",
            )}
          >
            {deltaPositive ? "↑" : "↓"} {Math.abs(delta).toFixed(1)}%
          </div>
        )}
      </div>
      <div className="flex items-end justify-between gap-3">
        <div
          className="flex items-baseline gap-1 text-4xl font-semibold tracking-[-0.03em] tabular-nums"
          style={valueColor ? { color: valueColor } : undefined}
        >
          {display !== undefined ? (
            // Preformatted text (decimals/unit/value-mapping already applied).
            <span>{display}</span>
          ) : (
            <>
              {prefix && <span className="text-muted-foreground text-2xl">{prefix}</span>}
              <motion.span>{animated}</motion.span>
              {suffix && <span className="text-muted-foreground text-xl">{suffix}</span>}
            </>
          )}
        </div>
        <div className="opacity-90" style={{ color: accent }}>
          <Spark data={spark} color={accent} />
        </div>
      </div>
    </motion.div>
  );
}
