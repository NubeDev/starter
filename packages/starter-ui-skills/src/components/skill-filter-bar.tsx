import * as React from "react";
import { cn } from "../lib/utils.js";

export type SkillsFilter = "all" | "approved" | "quarantined";

export interface SkillFilterBarProps
  extends React.HTMLAttributes<HTMLDivElement> {
  filter: SkillsFilter;
  onFilterChange: (f: SkillsFilter) => void;
  search: string;
  onSearchChange: (q: string) => void;
  counts?: Partial<Record<SkillsFilter, number>>;
}

const TABS: Array<{ id: SkillsFilter; label: string }> = [
  { id: "all", label: "All" },
  { id: "quarantined", label: "Quarantined" },
  { id: "approved", label: "Approved" },
];

export function SkillFilterBar({
  filter,
  onFilterChange,
  search,
  onSearchChange,
  counts,
  className,
  ...props
}: SkillFilterBarProps): React.ReactElement {
  return (
    <div
      data-slot="skill-filter-bar"
      className={cn(
        "flex flex-col gap-2 sm:flex-row sm:items-center",
        className,
      )}
      {...props}
    >
      <div
        role="tablist"
        aria-label="Filter skills"
        className="inline-flex items-center gap-1 rounded-lg border border-border/60 bg-muted/40 p-0.5"
      >
        {TABS.map((t) => {
          const active = filter === t.id;
          const count = counts?.[t.id];
          return (
            <button
              key={t.id}
              role="tab"
              type="button"
              aria-selected={active}
              onClick={() => onFilterChange(t.id)}
              className={cn(
                "inline-flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs font-medium transition",
                active
                  ? "bg-background text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              <span>{t.label}</span>
              {typeof count === "number" ? (
                <span
                  className={cn(
                    "rounded-full px-1.5 py-0.5 text-[10px]",
                    active
                      ? "bg-muted text-muted-foreground"
                      : "bg-background/60 text-muted-foreground",
                  )}
                >
                  {count}
                </span>
              ) : null}
            </button>
          );
        })}
      </div>
      <div className="relative flex-1">
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground"
          aria-hidden
        >
          <circle cx="11" cy="11" r="7" />
          <path d="M21 21l-4.3-4.3" />
        </svg>
        <input
          type="search"
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
          placeholder="Search skills…"
          className="w-full rounded-lg border border-border/60 bg-background/60 py-1.5 pl-8 pr-3 text-xs outline-none transition focus:border-ring/40 focus:ring-2 focus:ring-ring/20"
        />
      </div>
    </div>
  );
}
