// Forked from sql-studio (https://github.com/frectonz/sql-studio) — MIT.
// Upstream commit: 1a0736055a4647c18d0be19347e4325007c7bd52.
// Local edits: re-skinned to rubix tokens; data layer swapped to @nube/rubix-client-react.

import "react-data-grid/lib/styles.css";
import { useMemo, useState } from "react";

import { DataGrid } from "react-data-grid";
import { useDebounce } from "@uidotdev/usehooks";
import {
  ChevronDown,
  ChevronRight,
  Database,
  Key,
  Play,
  Search,
  ShieldX,
  Table as TableIcon,
  Terminal,
} from "lucide-react";

import { cn } from "../lib/utils";
import {
  useWarehouseErd,
  useWarehouseQuery,
} from "../hooks/use-warehouse";
import { useSql, useSqlDispatch } from "../providers/sql.provider";

import {
  Card,
  CardTitle,
  CardHeader,
  CardDescription,
} from "../components/ui/card";
import { Editor } from "../components/editor";
import { Input } from "../components/ui/input";
import { Toggle } from "../components/ui/toggle";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Skeleton } from "../components/ui/skeleton";
import { useTheme } from "../lib/theme";

/// Single-source-of-truth list of WHERE/ORDER/etc. snippets the
/// filters panel offers. Kept as a constant so the row of buttons
/// is purely data-driven.
const SQL_SNIPPETS: { label: string; insert: string }[] = [
  { label: "WHERE", insert: "\nWHERE column = 'value'" },
  { label: "AND", insert: "\n  AND column = 'value'" },
  { label: "ORDER BY", insert: "\nORDER BY column DESC" },
  { label: "GROUP BY", insert: "\nGROUP BY column" },
  { label: "LIMIT 100", insert: "\nLIMIT 100" },
  { label: "count(*)", insert: "count(*)" },
  { label: "now() - 1h", insert: "now() - INTERVAL 1 HOUR" },
  { label: "today", insert: "toDate(now())" },
];

export function Query() {
  const currentTheme = useTheme();

  const codeState = useSql();
  const setCodeState = useSqlDispatch();
  const code = useDebounce(codeState, 100);

  const [autoExecute, setAutoExecute] = useState(true);

  // Skip the request when the editor is empty. The backend rejects
  // `""` with `empty_query`, which would otherwise fire on every
  // mount because auto-execute defaults to on.
  const { data, error, refetch } = useWarehouseQuery(code, {
    enabled: autoExecute && code.trim().length > 0,
  });

  const grid = !data ? (
    !autoExecute && code && error ? (
      <Card>
        <CardHeader className="flex items-center">
          <ShieldX className="mb-2 h-12 w-12 text-red-400" />
          <CardTitle className="text-red-400">Error</CardTitle>
          <CardDescription className="text-red-400">
            Query didn't execute successfully.
          </CardDescription>
        </CardHeader>
      </Card>
    ) : (
      <Skeleton className="w-full h-[300px]" />
    )
  ) : data.columns.length === 0 ? (
    <Card>
      <CardHeader className="flex items-center">
        <Database className="mb-4 h-12 w-12 text-muted-foreground" />
        <CardTitle>Query Executed</CardTitle>
        <CardDescription>Returned no data</CardDescription>
      </CardHeader>
    </Card>
  ) : (
    <Card className="p-2 overflow-auto">
      <DataGrid
        defaultColumnOptions={{ resizable: true }}
        columns={data.columns.map((col) => ({
          key: col,
          name: col,
          renderCell: ({ row }: { row: Record<string, unknown> }) => (
            <div style={{ whiteSpace: "pre-wrap" }}>
              {String(row[col] ?? "")}
            </div>
          ),
        }))}
        rows={data.rows.map((row) =>
          (row as unknown[]).reduce<Record<string, unknown>>((acc, curr, i) => {
            acc[data.columns[i]] = curr;
            return acc;
          }, {}),
        )}
        rowHeight={(row) => {
          const maxLines = data.columns.reduce((max, col) => {
            const val = String((row as Record<string, unknown>)[col] ?? "");
            const lines = val.split(/\r?\n/).length;
            return Math.max(max, lines);
          }, 1);
          return Math.max(35, maxLines * 20 + 15);
        }}
        className={cn(currentTheme === "light" ? "rdg-light" : "rdg-dark")}
      />
    </Card>
  );

  return (
    <div className="grid gap-8">
      <div className="grid gap-4 md:grid-cols-[260px_1fr]">
        <FiltersPanel
          currentSql={codeState}
          onSetSql={(sql) => setCodeState({ type: "SET_SQL", data: sql })}
          onAppend={(text) =>
            setCodeState({
              type: "SET_SQL",
              data: codeState.endsWith("\n") || codeState.length === 0
                ? codeState + text.replace(/^\n/, "")
                : codeState + text,
            })
          }
        />

        <div className="grid gap-4 grid-cols-1 min-w-0">
          <Editor
            value={code}
            onChange={(val) => setCodeState({ type: "SET_SQL", data: val })}
          />

          <div className="flex gap-2 justify-between">
            <div className="flex gap-2">
              <Toggle
                size="sm"
                variant="outline"
                className="text-foreground"
                pressed={autoExecute}
                onPressedChange={(val) => setAutoExecute(val)}
                title={
                  autoExecute ? "Disable Auto Execute" : "Enable Auto Execute"
                }
              >
                <Terminal className="h-4 w-4" />
              </Toggle>

              {!autoExecute && (
                <Button size="sm" onClick={() => refetch()}>
                  <Play className="mr-2 h-4 w-4" /> Execute
                </Button>
              )}
            </div>
          </div>
        </div>
      </div>

      {grid}
    </div>
  );
}

// ---------------------------------------------------------------------------
// FiltersPanel — schema-aware "query builder helper" sidebar.
//
// Pulls the full database schema (table + column + type) from the
// metadata-only `/api/warehouse/explorer/erd` endpoint and exposes three
// flavours of one-click query construction:
//
//   1. Snippet pills (WHERE, ORDER BY, LIMIT 100, count(*), now()-1h…)
//      that append to the current editor buffer.
//   2. "Use" button per table → replaces the editor buffer with a
//      `SELECT * FROM \`table\` LIMIT 100;` starter.
//   3. Click any column row → appends the quoted column name at the
//      end of the editor buffer.
//
// Backed by `useWarehouseErd`, which scans
// `information_schema.columns` — no dictionary loads, safe even when
// external sources are missing.
// ---------------------------------------------------------------------------

interface FiltersPanelProps {
  currentSql: string;
  onSetSql: (sql: string) => void;
  onAppend: (text: string) => void;
}

function FiltersPanel({ currentSql: _current, onSetSql, onAppend }: FiltersPanelProps) {
  const { data, isLoading } = useWarehouseErd();
  const [search, setSearch] = useState("");
  const [open, setOpen] = useState<Record<string, boolean>>({});

  const filtered = useMemo(() => {
    const all = data?.tables ?? [];
    const q = search.trim().toLowerCase();
    if (!q) return all;
    return all
      .map((t) => {
        const tableMatch = t.name.toLowerCase().includes(q);
        const cols = t.columns.filter((c) =>
          c.name.toLowerCase().includes(q),
        );
        if (tableMatch) return t;
        if (cols.length > 0) return { ...t, columns: cols };
        return null;
      })
      .filter((t): t is NonNullable<typeof t> => t !== null);
  }, [data, search]);

  return (
    <Card className="p-3 h-fit md:sticky md:top-4 space-y-3 max-h-[calc(100vh-6rem)] overflow-y-auto">
      <div>
        <div className="flex items-center gap-2 mb-2">
          <Database className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium">Query helpers</span>
        </div>
        <div className="flex flex-wrap gap-1">
          {SQL_SNIPPETS.map((s) => (
            <Button
              key={s.label}
              variant="outline"
              size="sm"
              className="h-7 px-2 text-xs font-mono"
              onClick={() => onAppend(s.insert)}
              title={`Append: ${s.insert.trim()}`}
            >
              {s.label}
            </Button>
          ))}
        </div>
      </div>

      <div className="border-t pt-3">
        <div className="flex items-center gap-2 mb-2">
          <TableIcon className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium">
            Tables{" "}
            {data && (
              <span className="text-muted-foreground font-normal">
                ({data.tables.length})
              </span>
            )}
          </span>
        </div>

        <div className="relative mb-2">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Filter tables / columns…"
            className="h-7 pl-7 text-xs"
          />
        </div>

        {isLoading ? (
          <div className="space-y-1">
            <Skeleton className="h-6 w-full" />
            <Skeleton className="h-6 w-full" />
            <Skeleton className="h-6 w-full" />
          </div>
        ) : filtered.length === 0 ? (
          <p className="text-xs text-muted-foreground py-2">
            No tables match.
          </p>
        ) : (
          <ul className="space-y-0.5">
            {filtered.map((t) => {
              const isOpen = open[t.name] ?? !!search.trim();
              return (
                <li key={t.name}>
                  <div className="flex items-center gap-1 group">
                    <button
                      type="button"
                      onClick={() =>
                        setOpen((m) => ({ ...m, [t.name]: !isOpen }))
                      }
                      className="flex items-center gap-1 flex-1 min-w-0 text-left text-xs font-mono py-1 hover:text-foreground text-foreground/80"
                    >
                      {isOpen ? (
                        <ChevronDown className="h-3 w-3 shrink-0" />
                      ) : (
                        <ChevronRight className="h-3 w-3 shrink-0" />
                      )}
                      <span className="truncate">{t.name}</span>
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        onSetSql(
                          `SELECT *\nFROM \`${t.name}\`\nLIMIT 100;`,
                        )
                      }
                      className="opacity-0 group-hover:opacity-100 text-[10px] font-mono px-1.5 py-0.5 rounded border border-border hover:bg-muted shrink-0"
                      title={`SELECT * FROM ${t.name} LIMIT 100`}
                    >
                      use
                    </button>
                  </div>
                  {isOpen && (
                    <ul className="ml-4 border-l border-border/50 pl-2 py-0.5 space-y-0.5">
                      {t.columns.map((c) => (
                        <li key={c.name}>
                          <button
                            type="button"
                            onClick={() => onAppend(`\`${c.name}\``)}
                            className="flex items-center gap-1.5 w-full text-left text-[11px] font-mono py-0.5 px-1 rounded hover:bg-muted text-foreground/70 hover:text-foreground"
                            title={`Append \`${c.name}\` (${c.data_type})`}
                          >
                            {c.is_primary_key ? (
                              <Key className="h-2.5 w-2.5 text-amber-500 shrink-0" />
                            ) : (
                              <span className="w-2.5 shrink-0" />
                            )}
                            <span className="truncate">{c.name}</span>
                            <Badge
                              variant="secondary"
                              className="ml-auto text-[9px] px-1 py-0 font-mono font-normal shrink-0"
                            >
                              {shortType(c.data_type)}
                            </Badge>
                          </button>
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
    </Card>
  );
}

/// Collapse verbose ClickHouse type names (e.g. `Nullable(DateTime64(3))`)
/// into a short tag the column row can fit (`DT64`).
function shortType(t: string): string {
  // Strip Nullable() / LowCardinality() wrappers for display.
  const inner = t
    .replace(/^Nullable\((.*)\)$/, "$1")
    .replace(/^LowCardinality\((.*)\)$/, "$1");
  if (/^DateTime64/.test(inner)) return "DT64";
  if (/^DateTime/.test(inner)) return "DT";
  if (/^Date/.test(inner)) return "Date";
  if (/^U?Int\d+/.test(inner)) return inner.replace(/^U?Int/, (m) => m);
  if (/^Float\d+/.test(inner)) return inner;
  if (/^String|FixedString/.test(inner)) return "Str";
  if (/^Array\(/.test(inner)) return "Arr";
  if (/^Map\(/.test(inner)) return "Map";
  if (/^Tuple\(/.test(inner)) return "Tup";
  if (/^UUID$/.test(inner)) return "UUID";
  if (/^Bool$/.test(inner)) return "Bool";
  return inner.length > 8 ? inner.slice(0, 8) + "…" : inner;
}
