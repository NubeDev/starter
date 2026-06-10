import { useMemo, useState } from "react";
import {
  ChevronDown,
  ChevronRight,
  Columns3,
  Database,
  PanelLeftClose,
  PanelLeftOpen,
  Search,
  Table2,
} from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import { ScrollArea } from "@nube/starter-ui-kit/components/scroll-area";

import type { SchemaColumn, SchemaTable } from "@/api/types";
import { useDatasourceSchema } from "@/features/sql-editor";

// A persistent left-hand schema browser for Explore — the DataGrip/TablePlus/
// pgAdmin pattern. A real database (TimescaleDB, Citus, PostGIS, a partitioned
// schema) exposes hundreds or thousands of tables; a flat chip row or a
// transient popover buries that. A tree grouped by schema, with a filter box
// and collapse, scales to any size and any naming without blocklisting
// engine-specific internal names. Reads the same cached schema the editor's
// autocomplete uses, so it costs no extra request.

// A schema-qualified identifier, quoted only if it isn't a plain lowercase
// name — keeps the common case readable while staying correct for mixed-case
// or reserved names.
function ident(name: string): string {
  return /^[a-z_][a-z0-9_]*$/.test(name) ? name : `"${name}"`;
}

function tableRef(t: SchemaTable): string {
  return t.schema === "public"
    ? ident(t.name)
    : `${ident(t.schema)}.${ident(t.name)}`;
}

function peekQuery(t: SchemaTable): string {
  return `SELECT * FROM ${tableRef(t)} LIMIT 100;`;
}

// Which schemas a database author cares about least. Not used to *hide* — every
// table stays reachable — only to decide which groups start collapsed so the
// user's own `public` (and other application schemas) sit open at the top while
// engine internals stay folded away until searched for.
function isSystemSchema(schema: string): boolean {
  return schema.startsWith("_") || schema.startsWith("pg_");
}

export function SchemaSidebar({
  datasourceId,
  onPeek,
  collapsed,
  onToggleCollapsed,
}: {
  datasourceId: string | undefined;
  /** Run a peek (SELECT * … LIMIT 100) for the picked table. */
  onPeek: (sql: string) => void;
  /** When true, render only a thin rail with an expand button. */
  collapsed: boolean;
  onToggleCollapsed: () => void;
}) {
  const { data: schema, isLoading, isError } = useDatasourceSchema(datasourceId);
  const [filter, setFilter] = useState("");
  // Per-schema and per-table open state, keyed by name. Absent = use the
  // default (application schemas open, system schemas collapsed; tables closed).
  const [openSchemas, setOpenSchemas] = useState<Record<string, boolean>>({});
  const [openTables, setOpenTables] = useState<Record<string, boolean>>({});

  const groups = useMemo(() => {
    const q = filter.trim().toLowerCase();
    const bySchema = new Map<string, SchemaTable[]>();
    for (const t of schema?.tables ?? []) {
      const label = `${t.schema}.${t.name}`.toLowerCase();
      if (q && !label.includes(q)) continue;
      const list = bySchema.get(t.schema) ?? [];
      list.push(t);
      bySchema.set(t.schema, list);
    }
    // Application schemas first, then system schemas; each alphabetised.
    return [...bySchema.entries()]
      .sort(([a], [b]) => {
        const sysA = isSystemSchema(a) ? 1 : 0;
        const sysB = isSystemSchema(b) ? 1 : 0;
        return sysA - sysB || a.localeCompare(b);
      })
      .map(([name, items]) => ({
        name,
        system: isSystemSchema(name),
        items: items.slice().sort((a, b) => a.name.localeCompare(b.name)),
      }));
  }, [schema, filter]);

  const filtering = filter.trim().length > 0;
  const schemaOpen = (name: string, fallback: boolean) =>
    // While filtering, every matching group expands so results are visible
    // without clicking; otherwise honour explicit toggles, then the default.
    filtering || (openSchemas[name] ?? fallback);

  // Collapsed: a thin rail so the editor reclaims the width. One click reopens.
  if (collapsed) {
    return (
      <div className="glass flex h-full shrink-0 flex-col items-center rounded-xl p-1.5">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-8"
          onClick={onToggleCollapsed}
          title="Show tables"
          aria-label="Show tables"
        >
          <PanelLeftOpen className="size-4" />
        </Button>
      </div>
    );
  }

  return (
    <div className="glass flex h-full w-64 shrink-0 flex-col rounded-xl">
      <div className="flex items-center gap-1 border-b border-border/60 p-2">
        <div className="relative flex-1">
          <Search className="absolute start-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Search tables…"
            className="h-8 ps-7 text-sm"
            aria-label="Search tables"
            disabled={!datasourceId}
          />
        </div>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-8 shrink-0"
          onClick={onToggleCollapsed}
          title="Hide tables"
          aria-label="Hide tables"
        >
          <PanelLeftClose className="size-4" />
        </Button>
      </div>

      <ScrollArea className="min-h-0 flex-1">
        {!datasourceId ? (
          <Hint>Pick a datasource to browse its tables.</Hint>
        ) : isLoading ? (
          <Hint>Loading schema…</Hint>
        ) : isError ? (
          <Hint>Schema unavailable for this datasource.</Hint>
        ) : groups.length === 0 ? (
          <Hint>
            {filtering ? `No tables match “${filter}”.` : "No tables found."}
          </Hint>
        ) : (
          <div className="p-1">
            {groups.map((g) => {
              const open = schemaOpen(g.name, !g.system);
              return (
                <div key={g.name} className="mb-0.5">
                  <button
                    type="button"
                    onClick={() =>
                      setOpenSchemas((s) => ({
                        ...s,
                        [g.name]: !schemaOpen(g.name, !g.system),
                      }))
                    }
                    className="flex w-full items-center gap-1 rounded-sm px-1.5 py-1 text-start text-xs font-medium text-muted-foreground transition-colors hover:bg-primary/5 hover:text-foreground"
                  >
                    {open ? (
                      <ChevronDown className="size-3.5 shrink-0" />
                    ) : (
                      <ChevronRight className="size-3.5 shrink-0" />
                    )}
                    <Database className="size-3.5 shrink-0 text-muted-foreground/70" />
                    <span className="truncate">{g.name}</span>
                    <span className="ms-auto shrink-0 tabular-nums text-muted-foreground/60">
                      {g.items.length}
                    </span>
                  </button>

                  {open ? (
                    <div className="ms-2 border-s border-border/50 ps-1">
                      {g.items.map((t) => {
                        const key = `${t.schema}.${t.name}`;
                        const tOpen = openTables[key] ?? false;
                        return (
                          <div key={key}>
                            <div className="group flex items-center rounded-sm hover:bg-primary/10">
                              <button
                                type="button"
                                onClick={() =>
                                  setOpenTables((s) => ({ ...s, [key]: !tOpen }))
                                }
                                aria-label={
                                  tOpen ? "Hide columns" : "Show columns"
                                }
                                className="flex shrink-0 items-center px-0.5 py-1 text-muted-foreground/60 hover:text-foreground"
                              >
                                {tOpen ? (
                                  <ChevronDown className="size-3.5" />
                                ) : (
                                  <ChevronRight className="size-3.5" />
                                )}
                              </button>
                              <button
                                type="button"
                                onClick={() => onPeek(peekQuery(t))}
                                title={`Peek 100 rows from ${key}`}
                                className="flex min-w-0 flex-1 items-center gap-1.5 py-1 pe-1.5 text-start text-sm text-foreground"
                              >
                                <Table2 className="size-3.5 shrink-0 text-primary" />
                                <span className="truncate">{t.name}</span>
                              </button>
                            </div>
                            {tOpen ? (
                              <ColumnList columns={t.columns} />
                            ) : null}
                          </div>
                        );
                      })}
                    </div>
                  ) : null}
                </div>
              );
            })}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}

function ColumnList({ columns }: { columns: SchemaColumn[] }) {
  if (columns.length === 0) {
    return (
      <p className="ms-6 py-0.5 text-xs italic text-muted-foreground/60">
        no columns
      </p>
    );
  }
  return (
    <div className="ms-5 border-s border-border/40 ps-1.5">
      {columns.map((c) => (
        <div
          key={c.name}
          className="flex items-center gap-1.5 py-0.5 text-xs text-muted-foreground"
        >
          <Columns3 className="size-3 shrink-0 text-muted-foreground/50" />
          <span className="truncate">{c.name}</span>
          <span className="ms-auto shrink-0 font-mono text-[0.65rem] text-muted-foreground/50">
            {c.data_type}
          </span>
        </div>
      ))}
    </div>
  );
}

function Hint({ children }: { children: React.ReactNode }) {
  return <p className="p-3 text-xs text-muted-foreground">{children}</p>;
}
