// Dramatic horizontal hero strip — mimics the Canvas Due-Diligence
// reference: violet → cyan → amber → lime sweeping gradient with a
// slow hue-shift, page title overlay, and breadcrumb above.

import * as React from "react";
import { ChevronRight } from "lucide-react";

export function HeroGradient({
  title,
  subtitle,
  breadcrumb,
  rightEl,
}: {
  title: string;
  subtitle?: string;
  breadcrumb?: ReadonlyArray<string>;
  rightEl?: React.ReactNode;
}): React.ReactElement {
  return (
    <section
      aria-label="Page hero"
      className="relative overflow-hidden rounded-2xl border border-white/10"
      style={{ minHeight: 160 }}
    >
      {/* Gradient layer (animated) */}
      <div
        aria-hidden="true"
        className="nrg-hero-gradient absolute inset-0"
        style={{
          background:
            "linear-gradient(115deg, #6d28d9 0%, #0891b2 32%, #22d3ee 50%, #f59e0b 72%, #a3e635 100%)",
        }}
      />
      {/* Subtle noise / inner darkening for legibility */}
      <div
        aria-hidden="true"
        className="absolute inset-0"
        style={{
          background:
            "linear-gradient(180deg, rgba(15,15,35,0) 0%, rgba(15,15,35,0.10) 55%, rgba(15,15,35,0.55) 100%)",
        }}
      />

      {/* Foreground content */}
      <div className="relative z-10 flex h-full flex-col justify-between gap-3 p-5 sm:p-6">
        {breadcrumb && breadcrumb.length > 0 ? (
          <nav aria-label="Breadcrumb" className="flex items-center gap-1.5 text-[0.7rem] uppercase tracking-[0.18em] text-slate-100/85">
            {breadcrumb.map((crumb, i) => (
              <React.Fragment key={crumb + i}>
                {i > 0 ? <ChevronRight className="size-3 opacity-60" /> : null}
                <span className={i === breadcrumb.length - 1 ? "text-white font-semibold" : ""}>{crumb}</span>
              </React.Fragment>
            ))}
          </nav>
        ) : null}

        <div className="flex flex-wrap items-end justify-between gap-3">
          <div className="min-w-0">
            <h1
              className="text-3xl sm:text-4xl font-bold tracking-tight text-white drop-shadow-[0_2px_18px_rgba(0,0,0,0.45)]"
              style={{ fontFamily: '"Fira Sans", ui-sans-serif, system-ui, sans-serif' }}
            >
              {title}
            </h1>
            {subtitle ? (
              <p className="mt-1 text-sm text-white/85 max-w-2xl">{subtitle}</p>
            ) : null}
          </div>
          {rightEl ? <div className="shrink-0">{rightEl}</div> : null}
        </div>
      </div>
    </section>
  );
}
