import type { ReactNode } from "react"
import type { Icon } from "@tabler/icons-react"

import { cn } from "@/lib/utils"

export interface PageHeroProps {
  icon: Icon
  /** CSS color string or var() reference; e.g. `var(--accent-flows)`. */
  accent: string
  title: string
  description?: string
  /** Right-aligned actions (buttons, badges). */
  actions?: ReactNode
  className?: string
}

export function PageHero({
  icon: Icon,
  accent,
  title,
  description,
  actions,
  className,
}: PageHeroProps) {
  return (
    <div
      className={cn(
        "relative overflow-hidden rounded-2xl border border-border/60 bg-card/60 p-5 shadow-xs backdrop-blur-md",
        "before:pointer-events-none before:absolute before:inset-0 before:-z-0 before:opacity-70",
        "before:[background:radial-gradient(40rem_20rem_at_-10%_-40%,var(--accent-glow),transparent_60%),radial-gradient(30rem_18rem_at_110%_120%,var(--accent-glow-2),transparent_60%)]",
        className,
      )}
      style={
        {
          ["--accent-glow" as string]: `color-mix(in oklab, ${accent} 28%, transparent)`,
          ["--accent-glow-2" as string]: `color-mix(in oklab, ${accent} 14%, transparent)`,
        } as React.CSSProperties
      }
    >
      <div className="relative z-[1] flex items-start justify-between gap-4">
        <div className="flex items-start gap-4">
          <span
            aria-hidden
            className="grid size-12 shrink-0 place-items-center rounded-xl border border-border/60 bg-background/70 shadow-sm"
            style={{
              color: accent,
              boxShadow: `0 8px 24px -12px color-mix(in oklab, ${accent} 60%, transparent)`,
            }}
          >
            <Icon className="size-6" />
          </span>
          <div className="min-w-0">
            <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
            {description ? (
              <p className="mt-0.5 text-sm text-muted-foreground">
                {description}
              </p>
            ) : null}
          </div>
        </div>
        {actions ? (
          <div className="flex shrink-0 items-center gap-2">{actions}</div>
        ) : null}
      </div>
    </div>
  )
}
