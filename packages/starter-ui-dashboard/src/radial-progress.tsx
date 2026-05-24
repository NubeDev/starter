import { motion } from "motion/react";
import { cn } from "@nube/starter-ui-kit/lib/utils";

export interface RadialProgressProps {
  /** Progress value, 0–100. Clamped at render time. */
  value: number;
  /** Top-of-card label (e.g. "Battery"). Already localized. */
  label: string;
  /** Optional caption under the percentage (e.g. "12h remaining"). */
  subLabel?: string;
  /** Diameter in pixels. Default 180. */
  size?: number;
  /** Stroke width of the ring in pixels. Default 10. */
  stroke?: number;
  /** Stroke color for the filled arc. Accepts any CSS color; defaults
   * to `hsl(var(--primary))`. For a two-tone gradient pass a CSS
   * `linear-gradient(...)` via a custom SVG `<linearGradient>` if
   * needed. */
  accent?: string;
  /** Extra classes merged onto the card root. */
  className?: string;
}

/** Circular progress card with an animated stroke fill. Theme-agnostic
 * — uses shadcn `--card`, `--primary`, `--muted-foreground` tokens. */
export function RadialProgress({
  value,
  label,
  subLabel,
  size = 180,
  stroke = 10,
  accent = "hsl(var(--primary))",
  className,
}: RadialProgressProps) {
  const clamped = Math.max(0, Math.min(100, value));
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const offset = c - (clamped / 100) * c;

  return (
    <div
      className={cn(
        "bg-card text-card-foreground border-border relative overflow-hidden rounded-3xl border p-6",
        className,
      )}
    >
      <div className="text-muted-foreground text-[11px] font-medium uppercase tracking-[0.18em]">
        {label}
      </div>
      <div className="mt-4 flex items-center justify-center">
        <div className="relative" style={{ width: size, height: size }}>
          <svg width={size} height={size} className="-rotate-90">
            <circle
              cx={size / 2}
              cy={size / 2}
              r={r}
              fill="none"
              className="stroke-muted"
              strokeWidth={stroke}
            />
            <motion.circle
              cx={size / 2}
              cy={size / 2}
              r={r}
              fill="none"
              stroke={accent}
              strokeWidth={stroke}
              strokeLinecap="round"
              strokeDasharray={c}
              initial={{ strokeDashoffset: c }}
              whileInView={{ strokeDashoffset: offset }}
              viewport={{ once: true }}
              transition={{ duration: 1.6, ease: [0.22, 1, 0.36, 1] }}
            />
          </svg>
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <div className="text-4xl font-semibold tracking-tight tabular-nums">
              {clamped}
              <span className="text-muted-foreground text-xl">%</span>
            </div>
            {subLabel && (
              <div className="text-muted-foreground mt-1 text-[10px] uppercase tracking-[0.15em]">
                {subLabel}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
