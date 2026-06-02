// SchemaExplorer — a new, standalone page that re-skins the warehouse
// schema view to the polished schema-viewer reference. It is additive:
// the legacy `Explorer` shell and its `Overview` / `Tables` / `Query` /
// `Schema` views are left untouched.
//
// Layout, top to bottom:
//   - breadcrumb header (DB name + active section),
//   - an underline tab strip (only ERD is wired — the reference's CI /
//     Migrations / Webhooks tabs have no backend, so they are omitted
//     rather than faked),
//   - a sub-toolbar with table/relationship counts and an Export action,
//   - a split body: left schema tree + ERD canvas.
//
// Selection is lifted here so the tree and canvas stay in sync.

import { useMemo, useRef, useState } from "react";
import {
  Boxes,
  Database,
  Download,
  GitBranch,
  Loader2,
  Network,
  PanelLeftClose,
  PanelLeftOpen,
  ShieldX,
} from "lucide-react";

import { useWarehouseErd, useWarehouseStatus } from "../hooks/use-warehouse";
import { cn } from "../lib/utils";
import {
  SchemaErd,
  type SchemaErdHandle,
} from "../components/erd/schema-erd";
import { SchemaTree } from "./schema-tree";

export interface SchemaExplorerProps {
  /** Overrides the breadcrumb database label (defaults to the live file name). */
  title?: string;
}

export function SchemaExplorer({ title }: SchemaExplorerProps = {}) {
  const { data: erd, isLoading, isError, error } = useWarehouseErd();
  const { data: status } = useWarehouseStatus();
  const [selected, setSelected] = useState<string | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const erdRef = useRef<SchemaErdHandle>(null);

  const dbLabel = title ?? status?.file_name ?? "warehouse";

  const counts = useMemo(() => {
    if (!erd) return { tables: 0, relationships: 0 };
    return { tables: erd.tables.length, relationships: erd.relationships.length };
  }, [erd]);

  function handleSelect(name: string | null) {
    setSelected(name);
    if (name) erdRef.current?.focusNode(name);
  }

  function handleExport() {
    if (!erd) return;
    const blob = new Blob([JSON.stringify(erd, null, 2)], {
      type: "application/json;charset=utf-8",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${dbLabel}.schema.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setTimeout(() => URL.revokeObjectURL(url), 1_000);
  }

  return (
    <div className="flex h-screen w-full flex-col bg-background text-foreground">
      {/* Breadcrumb header */}
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-4">
        <Database className="h-4 w-4 text-muted-foreground" />
        <nav className="flex items-center gap-2 text-sm" aria-label="Breadcrumb">
          <span className="font-medium text-muted-foreground">{dbLabel}</span>
          <span className="text-border">/</span>
          <span className="font-semibold text-foreground">Schema</span>
        </nav>
      </header>

      {/* Tab strip — underline style. Only ERD is backed by real data. */}
      <div className="flex shrink-0 items-center gap-1 border-b border-border px-4">
        <TabButton active icon={Network} label="ERD" />
      </div>

      {/* Sub-toolbar */}
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-border px-4">
        <div className="flex items-center gap-4 text-xs text-muted-foreground">
          <span className="flex items-center gap-1.5">
            <Boxes className="h-3.5 w-3.5" />
            <span className="font-mono tabular-nums text-foreground">
              {counts.tables}
            </span>
            {counts.tables === 1 ? "table" : "tables"}
          </span>
          <span className="flex items-center gap-1.5">
            <GitBranch className="h-3.5 w-3.5" />
            <span className="font-mono tabular-nums text-foreground">
              {counts.relationships}
            </span>
            {counts.relationships === 1 ? "relationship" : "relationships"}
          </span>
        </div>
        <button
          type="button"
          onClick={handleExport}
          disabled={!erd}
          className="inline-flex items-center gap-1.5 rounded-md border border-border bg-card px-2.5 py-1.5 text-xs font-medium text-foreground transition-colors hover:bg-muted disabled:pointer-events-none disabled:opacity-50"
        >
          <Download className="h-3.5 w-3.5" />
          Export
        </button>
      </div>

      {/* Body */}
      <div className="flex min-h-0 flex-1">
        {/* Left tree */}
        <aside className="hidden w-72 shrink-0 border-r border-border md:block">
          {isLoading || !erd ? (
            <TreeSkeleton />
          ) : (
            <SchemaTree
              tables={erd.tables}
              relationships={erd.relationships}
              selected={selected}
              onSelect={handleSelect}
            />
          )}
        </aside>

        {/* Canvas */}
        <main className="relative min-w-0 flex-1">
          {isError ? (
            <CanvasMessage
              icon={ShieldX}
              tone="error"
              title="Failed to load schema"
              body={error?.message ?? "The schema endpoint returned an error."}
            />
          ) : isLoading || !erd ? (
            <CanvasMessage icon={Loader2} title="Loading schema…" spin />
          ) : erd.tables.length === 0 ? (
            <CanvasMessage
              icon={GitBranch}
              title="No tables found"
              body="The database has no tables to display in the schema diagram."
            />
          ) : (
            <SchemaErd
              data={erd}
              selectedNode={selected}
              onSelect={setSelected}
              handleRef={erdRef}
            />
          )}
        </main>
      </div>
    </div>
  );
}

function TabButton({
  active,
  icon: Icon,
  label,
}: {
  active?: boolean;
  icon: typeof Network;
  label: string;
}) {
  return (
    <button
      type="button"
      className={cn(
        "relative inline-flex items-center gap-2 px-2.5 py-2.5 text-sm font-medium transition-colors",
        active
          ? "text-foreground"
          : "text-muted-foreground hover:text-foreground",
      )}
    >
      <Icon className="h-4 w-4" />
      {label}
      {active && (
        <span className="absolute inset-x-0 -bottom-px h-0.5 rounded-full bg-foreground" />
      )}
    </button>
  );
}

function TreeSkeleton() {
  return (
    <div className="space-y-2 p-3">
      <div className="h-9 w-full animate-pulse rounded-lg bg-muted" />
      {Array.from({ length: 8 }).map((_, i) => (
        <div
          key={i}
          className="h-7 animate-pulse rounded-md bg-muted/60"
          style={{ width: `${70 + ((i * 7) % 25)}%` }}
        />
      ))}
    </div>
  );
}

function CanvasMessage({
  icon: Icon,
  title,
  body,
  tone,
  spin,
}: {
  icon: typeof Network;
  title: string;
  body?: string;
  tone?: "error";
  spin?: boolean;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 px-6 text-center">
      <Icon
        className={cn(
          "h-10 w-10",
          tone === "error" ? "text-destructive" : "text-muted-foreground",
          spin && "animate-spin",
        )}
      />
      <div>
        <p
          className={cn(
            "text-sm font-semibold",
            tone === "error" ? "text-destructive" : "text-foreground",
          )}
        >
          {title}
        </p>
        {body && (
          <p className="mt-1 max-w-sm text-xs text-muted-foreground">{body}</p>
        )}
      </div>
    </div>
  );
}
