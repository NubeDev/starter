import { motion, AnimatePresence } from "motion/react";
import { useEffect, useState } from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@nube/starter-ui-kit/lib/utils";

export interface ActivityItem {
  /** Stable key for animation reconciliation. */
  id: string;
  /** Lucide icon component (or any React component with a className
   * prop) shown in the left badge. */
  icon: LucideIcon;
  /** Already-localized headline. */
  title: string;
  /** Already-localized secondary line. */
  meta: string;
  /** Already-localized timestamp (e.g. "2m", "1h"). The first visible
   * item in the rotation falls back to `nowLabel` if provided. */
  time: string;
  /** CSS color (hex/hsl/var) for the icon tint. Defaults to current
   * card foreground. */
  accent?: string;
}

export interface ActivityFeedProps {
  /** Source data. The component cycles through them on a fixed timer
   * to feel "live"; pass a longer list for slower-feeling rotation. */
  items: ActivityItem[];
  /** Section heading (already localized). */
  title: string;
  /** Right-aligned status caption (e.g. "Streaming"). */
  streamingLabel?: string;
  /** Override the timestamp on the first visible row (e.g. "now"). */
  nowLabel?: string;
  /** Number of rows visible at once. Default 5. */
  visibleCount?: number;
  /** Rotation interval in milliseconds. Default 4500. Pass `0` to
   * disable rotation (useful for tests / Storybook). */
  intervalMs?: number;
  /** Extra classes merged onto the card root. */
  className?: string;
}

/** Auto-rotating "live activity" feed. Theme-agnostic — uses shadcn
 * `--card`, `--muted-foreground`, `--muted` tokens. Icon tinting is
 * driven by the optional `accent` field on each item. */
export function ActivityFeed({
  items,
  title,
  streamingLabel,
  nowLabel,
  visibleCount = 5,
  intervalMs = 4500,
  className,
}: ActivityFeedProps) {
  const [start, setStart] = useState(0);

  useEffect(() => {
    if (!items.length || intervalMs <= 0) return;
    const t = setInterval(() => setStart((s) => (s + 1) % items.length), intervalMs);
    return () => clearInterval(t);
  }, [items.length, intervalMs]);

  if (!items.length) return null;

  const take = Math.min(visibleCount, items.length);
  const visible = Array.from({ length: take }, (_, i) => {
    const item = items[(start + i) % items.length];
    if (!item) throw new Error("unreachable: items.length asserted above");
    return item;
  });

  return (
    <div
      className={cn(
        "bg-card text-card-foreground border-border relative overflow-hidden rounded-3xl border p-6",
        className,
      )}
    >
      <div className="mb-6 flex items-center justify-between">
        <div className="text-muted-foreground text-[11px] font-medium uppercase tracking-[0.18em]">
          {title}
        </div>
        {streamingLabel && (
          <span className="text-primary flex items-center gap-1.5 text-[10px] uppercase tracking-[0.15em]">
            <span className="relative flex h-1.5 w-1.5">
              <span className="bg-primary absolute inline-flex h-full w-full animate-ping rounded-full opacity-75" />
              <span className="bg-primary relative inline-flex h-1.5 w-1.5 rounded-full" />
            </span>
            {streamingLabel}
          </span>
        )}
      </div>

      <ul className="relative space-y-1">
        <AnimatePresence initial={false} mode="popLayout">
          {visible.map((item, i) => {
            const Icon = item.icon;
            const tint = item.accent ?? "currentColor";
            return (
              <motion.li
                key={item.id + i}
                layout
                initial={{ opacity: 0, y: -12 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, x: 30 }}
                transition={{ duration: 0.55, ease: [0.22, 1, 0.36, 1] }}
                className="hover:bg-muted/40 group flex cursor-default items-center gap-4 rounded-2xl px-3 py-3 transition-colors"
              >
                <div
                  className="ring-border flex h-9 w-9 items-center justify-center rounded-xl ring-1"
                  style={{ color: tint, background: `color-mix(in oklab, ${tint} 10%, transparent)` }}
                >
                  <Icon className="h-4 w-4" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium">{item.title}</div>
                  <div className="text-muted-foreground truncate text-xs">{item.meta}</div>
                </div>
                <div className="text-muted-foreground shrink-0 text-[11px] tabular-nums">
                  {i === 0 && nowLabel ? nowLabel : item.time}
                </div>
              </motion.li>
            );
          })}
        </AnimatePresence>
      </ul>
    </div>
  );
}
