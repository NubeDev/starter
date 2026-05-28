// Forked from sql-studio (https://github.com/frectonz/sql-studio) — MIT.
// Upstream commit: 1a0736055a4647c18d0be19347e4325007c7bd52.
// Local edits: re-skinned to rubix tokens; data layer swapped to @nube/rubix-client-react.
//
// Upstream selects the active table via TanStack search params
// (`?table=…`). In this host the explorer doesn't own the URL, so the
// active table lives in local state instead. Visual layout unchanged.

import "react-data-grid/lib/styles.css";

import { useState } from "react";
import {
  ChevronDown,
  ChevronRight,
  Download,
  FileJson,
  FileText,
  HardDrive,
  DatabaseZap,
  Search,
  TableProperties,
  Table as TableIcon,
} from "lucide-react";
import { DataGrid } from "react-data-grid";
import { CodeBlock, irBlack as CodeDarkTheme } from "react-code-blocks";
import {
  fetchJson,
  readCsrfHeader,
  type StarterError,
} from "@nube/starter-client-ts";
import { useStarterClient } from "@nube/starter-client-react";

import { cn } from "../lib/utils";
import {
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "../components/ui/dropdown-menu";
import { Input } from "../components/ui/input";
import { Skeleton } from "../components/ui/skeleton";
import { useTheme } from "../lib/theme";
import {
  useWarehouseTable,
  useWarehouseTableData,
  useWarehouseTables,
} from "../hooks/use-warehouse";
import { InfoCard, InfoCardProps } from "../components/info-card";
import type { Query as QueryResult, Table as TableMeta } from "../api";

// Derive a namespace group label from a table name.
// Double-underscore acts as a namespace separator (e.g. `com_acme__orders` → `com_acme`).
// Leading-underscore tables (migration internals) are grouped by their first two segments.
// Plain names with a single underscore group by the first segment; bare names stand alone.
function tableGroup(name: string): string {
  const dbl = name.indexOf("__");
  if (dbl > 0) return name.slice(0, dbl);
  if (name.startsWith("_")) {
    const parts = name.split("_").filter(Boolean);
    return "_" + parts.slice(0, 2).join("_");
  }
  const under = name.indexOf("_");
  return under > 0 ? name.slice(0, under) : name;
}

export function Tables() {
  const [selected, setSelected] = useState<string | undefined>(undefined);
  const [filter, setFilter] = useState("");
  // Track which groups are collapsed; all start open.
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const { data } = useWarehouseTables();

  if (!data) return <TablesSkeleton />;

  if (data.tables.length === 0)
    return (
      <Card>
        <CardHeader className="flex items-center">
          <TableIcon className="mb-4 h-12 w-12 text-muted-foreground" />
          <CardTitle>No Tables Found</CardTitle>
          <CardDescription>The database has no tables.</CardDescription>
        </CardHeader>
      </Card>
    );

  const needle = filter.trim().toLowerCase();
  const filtered = needle
    ? data.tables.filter((t) => t.name.toLowerCase().includes(needle))
    : data.tables;

  const selectedTable =
    (selected && data.tables.some(({ name }) => name === selected)
      ? selected
      : filtered[0]?.name) ?? data.tables[0].name;

  // Build ordered group map: preserve the order groups first appear.
  const groupMap = new Map<string, typeof filtered>();
  for (const t of filtered) {
    const g = tableGroup(t.name);
    if (!groupMap.has(g)) groupMap.set(g, []);
    groupMap.get(g)!.push(t);
  }
  // Only show groups if there are multiple distinct ones and no active filter.
  const useGroups = !needle && groupMap.size > 1;

  function toggleGroup(g: string) {
    setCollapsed((prev) => {
      const next = new Set(prev);
      next.has(g) ? next.delete(g) : next.add(g);
      return next;
    });
  }

  return (
    <div className="flex flex-col gap-4 lg:flex-row lg:items-start">
      {/* Left rail — searchable vertical list. Fixed height at every
       * viewport so it always scrolls independently of the detail panel. */}
      <aside className="flex w-full shrink-0 flex-col gap-2 lg:sticky lg:top-16 lg:w-72"
             style={{ height: "calc(100vh - 8rem)" }}>
        <div className="relative shrink-0">
          <Search
            className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden="true"
          />
          <Input
            type="search"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder={`Filter ${data.tables.length} tables…`}
            aria-label="Filter tables"
            className="h-9 pl-8"
          />
        </div>
        <div
          role="listbox"
          aria-label="Tables"
          className="min-h-0 flex-1 overflow-y-auto rounded-md border bg-card"
        >
          {filtered.length === 0 ? (
            <div className="px-3 py-6 text-center text-xs text-muted-foreground">
              No tables match &ldquo;{filter}&rdquo;
            </div>
          ) : useGroups ? (
            Array.from(groupMap.entries()).map(([group, tables]) => {
              const open = !collapsed.has(group);
              return (
                <div key={group}>
                  <button
                    type="button"
                    onClick={() => toggleGroup(group)}
                    className="flex w-full cursor-pointer items-center gap-1.5 border-b px-2 py-1.5 text-left transition-colors hover:bg-secondary/40"
                  >
                    {open ? (
                      <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" />
                    ) : (
                      <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground" />
                    )}
                    <span className="min-w-0 truncate font-mono text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                      {group}
                    </span>
                    <Badge
                      variant="outline"
                      className="ml-auto shrink-0 px-1 py-0 text-[10px] font-normal tabular-nums"
                    >
                      {tables.length}
                    </Badge>
                  </button>
                  {open &&
                    tables.map((t) => (
                      <TableRow
                        key={t.name}
                        table={t}
                        active={t.name === selectedTable}
                        onSelect={setSelected}
                        indent
                      />
                    ))}
                </div>
              );
            })
          ) : (
            filtered.map((t) => (
              <TableRow
                key={t.name}
                table={t}
                active={t.name === selectedTable}
                onSelect={setSelected}
              />
            ))
          )}
        </div>
      </aside>
      <div className="min-w-0 flex-1 overflow-y-auto" style={{ height: "calc(100vh - 8rem)" }}>
        <Table name={selectedTable} />
      </div>
    </div>
  );
}

type TableRowProps = {
  table: { name: string; count: number };
  active: boolean;
  onSelect: (name: string) => void;
  indent?: boolean;
};

function TableRow({ table: t, active, onSelect, indent }: TableRowProps) {
  return (
    <button
      type="button"
      role="option"
      aria-selected={active}
      onClick={() => onSelect(t.name)}
      className={cn(
        "group flex w-full items-center justify-between gap-2 border-l-2 border-transparent text-left text-sm transition-colors",
        "hover:bg-secondary/60",
        indent ? "px-3 py-1" : "px-3 py-1.5",
        active
          ? "border-l-primary bg-secondary font-medium text-foreground"
          : "text-muted-foreground",
      )}
    >
      <span className={cn("truncate font-mono", indent ? "text-[11.5px]" : "text-[12.5px]")}>
        {t.name}
      </span>
      <Badge
        variant={active ? "default" : "secondary"}
        className="shrink-0 px-1.5 py-0 text-[10px] font-normal tabular-nums"
      >
        {t.count.toLocaleString()}
      </Badge>
    </button>
  );
}

function TablesSkeleton() {
  return (
    <div className="flex flex-col gap-4 lg:flex-row lg:items-start">
      <Skeleton className="h-[calc(100vh-8rem)] w-full shrink-0 lg:w-72" />
      <Skeleton className="h-[calc(100vh-8rem)] flex-1" />
    </div>
  );
}

type Props = {
  name: string;
};
function Table({ name }: Props) {
  const currentTheme = useTheme();
  const { data } = useWarehouseTable(name);

  if (!data) return <TableSkeleton />;

  const isVirtual = data.sql?.startsWith("CREATE VIRTUAL TABLE") ?? false;

  const cards: InfoCardProps[] = [
    {
      title: "ROW COUNT",
      value: data.row_count.toLocaleString(),
      description: "The number of rows in the table.",
      icon: TableIcon,
    },
    {
      title: "INDEXES",
      value: data.index_count.toLocaleString(),
      description: "The number of indexes in the table.",
      icon: DatabaseZap,
    },
    {
      title: "COLUMNS",
      value: data.column_count.toLocaleString(),
      description: "The number of columns in the table.",
      icon: TableProperties,
    },
    {
      title: "TABLE SIZE",
      value: data.table_size,
      description: "The size of the table on disk.",
      icon: HardDrive,
    },
  ];

  return (
    <div className="flex flex-1 flex-col gap-4 md:gap-8">
      <div className="flex flex-wrap items-end justify-between gap-3 px-2 border-b pb-2">
        <h2 className="text-foreground scroll-m-20 text-3xl font-semibold tracking-tight first:mt-0 flex items-center gap-3">
          {data.name}
          {isVirtual && <Badge variant="secondary">Virtual Table</Badge>}
        </h2>
        <ExportMenu meta={data} />
      </div>

      <div className="grid gap-4 md:grid-cols-2 md:gap-8 lg:grid-cols-4">
        {cards.map((card, i) => (
          <InfoCard
            key={i}
            title={card.title}
            value={card.value}
            description={card.description}
            icon={card.icon}
          />
        ))}
      </div>

      {data.sql && (
        <Card className="font-mono text-sm">
          <CodeBlock
            text={data.sql}
            language="sql"
            theme={currentTheme === "dark" ? CodeDarkTheme : undefined}
            showLineNumbers={false}
            customStyle={{
              FontFace: "JetBrains Mono",
              padding: "10px",
              backgroundColor: currentTheme === "dark" ? "#091813" : "#f5faf9",
              borderRadius: "10px",
            }}
          />
        </Card>
      )}

      <Card className="p-2">
        <TableDataView name={data.name} />
      </Card>
    </div>
  );
}

function TableSkeleton() {
  return (
    <div className="flex flex-1 flex-col gap-4 md:gap-8">
      <div className="flex flex-col gap-2">
        <Skeleton className="w-[50vw] h-[50px]" />
        <span className="border-b" />
      </div>

      <div className="grid gap-4 md:grid-cols-2 md:gap-8 lg:grid-cols-4">
        <Skeleton className="h-[100px]" />
        <Skeleton className="h-[100px]" />
        <Skeleton className="h-[100px]" />
        <Skeleton className="h-[100px]" />
      </div>

      <Skeleton className="h-[400px]" />
      <Skeleton className="h-[400px]" />
    </div>
  );
}

function isAtBottom({ currentTarget }: React.UIEvent<HTMLDivElement>): boolean {
  return (
    currentTarget.scrollTop + 10 >=
    currentTarget.scrollHeight - currentTarget.clientHeight
  );
}

type TableDataProps = {
  name: string;
};
function TableDataView({ name }: TableDataProps) {
  const currentTheme = useTheme();
  const { isLoading, data, fetchNextPage, hasNextPage } =
    useWarehouseTableData(name);

  if (!data) return <Skeleton className="h-[400px]" />;

  function handleScroll(event: React.UIEvent<HTMLDivElement>) {
    if (isLoading || !isAtBottom(event) || !hasNextPage) return;
    fetchNextPage();
  }

  const columns = data.pages[0].columns.map((col) => ({ key: col, name: col }));

  const grouped = data.pages.map((page) =>
    page.rows.map((row) =>
      (row as unknown[]).reduce<Record<string, unknown>>((acc, curr, i) => {
        acc[page.columns[i]] = curr;
        return acc;
      }, {}),
    ),
  ) as never[][];
  const rows = ([] as never[]).concat(...grouped);

  return (
    <DataGrid
      rows={rows}
      columns={columns}
      onScroll={handleScroll}
      defaultColumnOptions={{ resizable: true }}
      className={cn(currentTheme === "light" ? "rdg-light" : "rdg-dark")}
    />
  );
}

// ---------------------------------------------------------------------------
// ExportMenu — schema + data export for the current table.
//
// Schema exports are synchronous (built from the already-loaded
// `Table` metadata). Data exports run a one-off `SELECT * FROM
// <table> LIMIT N` against `/api/warehouse/explorer/query` (the
// backend executes every statement under a `READ ONLY DEFERRABLE`
// txn, so mutations are engine-rejected) and serialise the result
// client-side.
//
// The row cap is deliberate — pulling unbounded warehouse tables
// into a browser tab is a foot-gun. 10_000 is the upstream sql-studio
// default and is plenty for the explore-then-grab-a-sample workflow
// the user is after; larger exports should go through the `query`
// editor + the warehouse REST surface.
// ---------------------------------------------------------------------------

const EXPORT_ROW_CAP = 10_000;

interface ExportMenuProps {
  meta: TableMeta;
}

function ExportMenu({ meta }: ExportMenuProps) {
  const starter = useStarterClient();
  const [busy, setBusy] = useState<null | string>(null);

  async function exportData(format: "csv" | "json") {
    setBusy(format);
    try {
      const sql = `SELECT * FROM \`${meta.name}\` LIMIT ${EXPORT_ROW_CAP}`;
      const result = await fetchJson<QueryResult>(
        starter,
        `/api/warehouse/explorer/query`,
        {
          method: "POST",
          headers: {
            "content-type": "application/json",
            ...readCsrfHeader(),
          },
          body: JSON.stringify({ sql }),
        },
      );
      if (format === "csv") {
        download(
          `${meta.name}.csv`,
          "text/csv;charset=utf-8",
          rowsToCsv(result.columns, result.rows as unknown[][]),
        );
      } else {
        download(
          `${meta.name}.json`,
          "application/json;charset=utf-8",
          JSON.stringify(rowsToObjects(result.columns, result.rows as unknown[][]), null, 2),
        );
      }
    } catch (err) {
      const message = (err as StarterError)?.message ?? String(err);
      // eslint-disable-next-line no-alert
      alert(`Export failed: ${message}`);
    } finally {
      setBusy(null);
    }
  }

  function exportSchemaJson() {
    const payload = {
      name: meta.name,
      row_count: meta.row_count,
      column_count: meta.column_count,
      index_count: meta.index_count,
      table_size: meta.table_size,
      sql: meta.sql,
    };
    download(
      `${meta.name}.schema.json`,
      "application/json;charset=utf-8",
      JSON.stringify(payload, null, 2),
    );
  }

  function exportSchemaSql() {
    if (!meta.sql) return;
    download(`${meta.name}.sql`, "text/plain;charset=utf-8", `${meta.sql};\n`);
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" disabled={busy !== null}>
          <Download className="mr-2 h-4 w-4" />
          {busy ? `Exporting (${busy.toUpperCase()})…` : "Export"}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel className="text-xs font-normal text-muted-foreground">
          Data (first {EXPORT_ROW_CAP.toLocaleString()} rows)
        </DropdownMenuLabel>
        <DropdownMenuItem onClick={() => exportData("csv")}>
          <FileText className="mr-2 h-4 w-4" />
          CSV
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => exportData("json")}>
          <FileJson className="mr-2 h-4 w-4" />
          JSON
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuLabel className="text-xs font-normal text-muted-foreground">
          Schema
        </DropdownMenuLabel>
        <DropdownMenuItem onClick={exportSchemaJson}>
          <FileJson className="mr-2 h-4 w-4" />
          Columns &amp; stats (JSON)
        </DropdownMenuItem>
        <DropdownMenuItem
          onClick={exportSchemaSql}
          disabled={!meta.sql}
        >
          <FileText className="mr-2 h-4 w-4" />
          CREATE TABLE (SQL)
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/// Serialise a 2-D result set to RFC-4180 CSV. Wrap any cell that
/// contains `"`, `,`, `\n`, or `\r` in double quotes and double the
/// inner quotes. `null` / `undefined` become empty fields.
function rowsToCsv(columns: string[], rows: unknown[][]): string {
  const out: string[] = [columns.map(csvCell).join(",")];
  for (const row of rows) {
    out.push(row.map(csvCell).join(","));
  }
  return out.join("\r\n") + "\r\n";
}

function csvCell(v: unknown): string {
  if (v === null || v === undefined) return "";
  const s = typeof v === "string" ? v : JSON.stringify(v);
  return /[",\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

function rowsToObjects(
  columns: string[],
  rows: unknown[][],
): Record<string, unknown>[] {
  return rows.map((row) =>
    row.reduce<Record<string, unknown>>((acc, cell, i) => {
      acc[columns[i]] = cell;
      return acc;
    }, {}),
  );
}

function download(filename: string, mime: string, body: string) {
  const blob = new Blob([body], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  // Defer revoke so Safari's download stream isn't truncated.
  setTimeout(() => URL.revokeObjectURL(url), 1_000);
}
