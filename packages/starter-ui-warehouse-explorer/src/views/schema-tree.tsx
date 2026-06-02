// Left schema tree for the new Schema Explorer page.
//
// Mirrors the reference's collapsible tree: a search box with a `/`
// focus hint, then Tables → <table> → Columns. Each table row is
// selectable (focuses the matching ERD node); each column row shows a
// key glyph and a short type chip. Driven entirely off the ERD payload
// (tables + columns + relationships) — no invented index data.

import { useMemo, useRef, useState } from "react";
import {
  ChevronDown,
  ChevronRight,
  KeyRound,
  Link2,
  Search,
  Table2,
} from "lucide-react";

import { cn } from "../lib/utils";

type ErdColumn = {
  name: string;
  data_type: string;
  nullable: boolean;
  is_primary_key: boolean;
};

type ErdTable = { name: string; columns: ErdColumn[] };

type ErdRelationship = {
  from_table: string;
  from_column: string;
  to_table: string;
  to_column: string;
};

type Props = {
  tables: ErdTable[];
  relationships: ErdRelationship[];
  selected: string | null;
  onSelect: (name: string) => void;
};

export function SchemaTree({ tables, relationships, selected, onSelect }: Props) {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<Set<string>>(new Set());
  const searchRef = useRef<HTMLInputElement>(null);

  // Columns that take part in a relationship → link glyph in the tree.
  const fkColumns = useMemo(() => {
    const m = new Map<string, Set<string>>();
    for (const r of relationships) {
      if (!m.has(r.from_table)) m.set(r.from_table, new Set());
      m.get(r.from_table)!.add(r.from_column);
    }
    return m;
  }, [relationships]);

  const needle = query.trim().toLowerCase();
  const filtered = useMemo(() => {
    if (!needle) return tables;
    return tables
      .map((t) => {
        if (t.name.toLowerCase().includes(needle)) return t;
        const cols = t.columns.filter((c) =>
          c.name.toLowerCase().includes(needle),
        );
        return cols.length ? { ...t, columns: cols } : null;
      })
      .filter((t): t is ErdTable => t !== null);
  }, [tables, needle]);

  // While searching, auto-expand matches so hits are visible.
  const isOpen = (name: string) => (needle ? true : open.has(name));
  const toggle = (name: string) =>
    setOpen((prev) => {
      const next = new Set(prev);
      next.has(name) ? next.delete(name) : next.add(name);
      return next;
    });

  return (
    <div className="flex h-full flex-col">
      {/* Search */}
      <div className="shrink-0 p-3">
        <div className="relative">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            ref={searchRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") setQuery("");
            }}
            placeholder="Search…"
            aria-label="Search schema"
            className="h-9 w-full rounded-lg border border-input bg-background pl-8 pr-8 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground focus:border-ring focus:ring-2 focus:ring-ring/30"
          />
          <kbd className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 rounded border border-border bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
            /
          </kbd>
        </div>
      </div>

      {/* Tree */}
      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
        <div className="flex items-center gap-1.5 px-2 py-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
          <Table2 className="h-3.5 w-3.5" />
          Tables
          <span className="ml-auto font-mono text-[10px] font-normal tabular-nums text-muted-foreground/70">
            {filtered.length}
          </span>
        </div>

        {filtered.length === 0 ? (
          <p className="px-3 py-6 text-center text-xs text-muted-foreground">
            No tables match &ldquo;{query}&rdquo;
          </p>
        ) : (
          <ul className="space-y-0.5">
            {filtered.map((t) => {
              const expanded = isOpen(t.name);
              const active = t.name === selected;
              const fk = fkColumns.get(t.name);
              return (
                <li key={t.name}>
                  <div
                    className={cn(
                      "group flex items-center gap-1 rounded-md pr-2 transition-colors",
                      active
                        ? "bg-primary/10 text-foreground"
                        : "hover:bg-muted/60",
                    )}
                  >
                    <button
                      type="button"
                      onClick={() => toggle(t.name)}
                      aria-label={expanded ? "Collapse" : "Expand"}
                      className="flex h-7 w-6 shrink-0 items-center justify-center text-muted-foreground"
                    >
                      {expanded ? (
                        <ChevronDown className="h-3.5 w-3.5" />
                      ) : (
                        <ChevronRight className="h-3.5 w-3.5" />
                      )}
                    </button>
                    <button
                      type="button"
                      onClick={() => onSelect(t.name)}
                      aria-current={active}
                      className={cn(
                        "flex min-w-0 flex-1 items-center gap-2 py-1 text-left",
                      )}
                    >
                      <Table2
                        className={cn(
                          "h-3.5 w-3.5 shrink-0",
                          active ? "text-primary" : "text-muted-foreground",
                        )}
                      />
                      <span
                        className={cn(
                          "truncate font-mono text-[13px]",
                          active
                            ? "font-medium text-foreground"
                            : "text-foreground/85",
                        )}
                      >
                        {t.name}
                      </span>
                    </button>
                  </div>

                  {expanded && (
                    <ul className="ml-[1.4rem] border-l border-border/60 pl-1">
                      {t.columns.map((c) => (
                        <li key={c.name}>
                          <div className="flex items-center gap-2 rounded-md py-1 pl-2 pr-2 text-[12px] hover:bg-muted/40">
                            {c.is_primary_key ? (
                              <KeyRound className="h-3 w-3 shrink-0 text-amber-500" />
                            ) : fk?.has(c.name) ? (
                              <Link2 className="h-3 w-3 shrink-0 text-sky-500" />
                            ) : (
                              <span className="w-3 shrink-0" />
                            )}
                            <span className="truncate font-mono text-foreground/80">
                              {c.name}
                            </span>
                            <span className="ml-auto shrink-0 truncate font-mono text-[10px] text-muted-foreground">
                              {c.data_type}
                            </span>
                          </div>
                        </li>
                      ))}
                    </ul>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
