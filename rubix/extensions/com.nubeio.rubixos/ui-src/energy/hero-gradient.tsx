// Theme-aware hero band. A subtle horizontal tint over the host's
// `--color-card` surface, driven by the host brand tokens
// (`--color-primary`, `--color-aqua`, `--color-leaf`). Both light
// and dark themes flow through unchanged — no hardcoded slate/violet.
//
// The hero hosts:
//   - breadcrumb (muted-foreground)
//   - title + subtitle
//   - section pill strip (Overview / Solar / Wind / ...)
//   - optional right-aligned status element
//
// Drama comes from typography + content density, not from a rainbow
// gradient that fights the user's chosen theme.

import * as React from "react";
import { ChevronRight } from "lucide-react";

import { PillBtn } from "../dashboard/prims";

export type EnergySection =
  | "overview"
  | "solar"
  | "wind"
  | "water"
  | "hydro"
  | "storage";

const SECTIONS: ReadonlyArray<{ id: EnergySection; label: string }> = [
  { id: "overview", label: "Overview" },
  { id: "solar",    label: "Solar"    },
  { id: "wind",     label: "Wind"     },
  { id: "water",    label: "Water"    },
  { id: "hydro",    label: "Hydro"    },
  { id: "storage",  label: "Storage"  },
];

export function HeroGradient({
  title,
  subtitle,
  breadcrumb,
  section,
  onSelectSection,
  rightEl,
}: {
  title: string;
  subtitle?: string;
  breadcrumb?: ReadonlyArray<string>;
  section: EnergySection;
  onSelectSection: (id: EnergySection) => void;
  rightEl?: React.ReactNode;
}): React.ReactElement {
  return (
    <section
      aria-label="Page hero"
      className="ext-glass relative overflow-hidden p-5 sm:p-6"
    >
      {/* Theme-tinted band — subtle gradient over the card surface,
       * built from host brand tokens. Deepens on dark, softens on
       * light. Stays out of the way of foreground text. */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 bg-gradient-to-r from-[var(--color-primary)]/10 via-[var(--color-aqua)]/8 to-[var(--color-leaf)]/10"
      />

      <div className="relative z-10 flex flex-col gap-3">
        {breadcrumb && breadcrumb.length > 0 ? (
          <nav
            aria-label="Breadcrumb"
            className="flex items-center gap-1.5 text-[0.7rem] uppercase tracking-[0.18em] text-muted-foreground"
          >
            {breadcrumb.map((crumb, i) => (
              <React.Fragment key={crumb + i}>
                {i > 0 ? <ChevronRight className="size-3 opacity-60" /> : null}
                <span
                  className={
                    i === breadcrumb.length - 1
                      ? "font-semibold text-foreground"
                      : ""
                  }
                >
                  {crumb}
                </span>
              </React.Fragment>
            ))}
          </nav>
        ) : null}

        <div className="flex flex-wrap items-end justify-between gap-3">
          <div className="min-w-0">
            <h1 className="text-3xl font-bold tracking-tight text-foreground sm:text-4xl">
              {title}
            </h1>
            {subtitle ? (
              <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
                {subtitle}
              </p>
            ) : null}
          </div>
          {rightEl ? <div className="shrink-0">{rightEl}</div> : null}
        </div>

        {/* Section pill strip. Active pill gets ext-glass--accent so
         * it shares the host's primary-token highlight. */}
        <div
          role="tablist"
          aria-label="Energy sections"
          className="flex flex-wrap items-center gap-1.5 pt-1"
        >
          {SECTIONS.map((s) => {
            const active = section === s.id;
            return (
              <span
                key={s.id}
                role="tab"
                aria-selected={active}
                className={active ? "ext-glass--accent rounded-full" : ""}
              >
                <PillBtn active={active} onClick={() => onSelectSection(s.id)}>
                  {s.label}
                </PillBtn>
              </span>
            );
          })}
        </div>
      </div>
    </section>
  );
}
