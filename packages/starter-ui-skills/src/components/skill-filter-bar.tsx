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
        "flex flex-wrap items-center gap-2",
        className,
      )}
      {...props}
    >
      <div
        role="tablist"
        aria-label="Filter skills"
        className="inline-flex items-center gap-0.5 rounded-md border border-border/60 bg-muted/30 p-0.5"
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
                "inline-flex h-6 items-center gap-1.5 rounded px-2 text-[11px] font-medium transition-colors",
                active
                  ? "bg-background text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              <span>{t.label}</span>
              {typeof count === "number" ? (
                <span className="text-[10px] tabular-nums text-muted-foreground">
                  {count}
                </span>
              ) : null}
            </button>
          );
        })}
      </div>
      <div className="relative ml-auto w-full max-w-[14rem] sm:w-auto">
        <input
          type="search"
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
          placeholder="Search skills…"
          className="h-7 w-full rounded-md border border-border/60 bg-background/60 px-2.5 text-[11px] outline-none placeholder:text-muted-foreground/70 focus:border-ring/40 focus:ring-2 focus:ring-ring/20"
        />
      </div>
    </div>
  );
}
