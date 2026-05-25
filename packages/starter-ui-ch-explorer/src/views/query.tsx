// Forked from sql-studio (MIT) — https://github.com/frectonz/sql-studio
// Original copyright (c) frectonz. See NOTICES.md.
//
// Query view — Monaco SQL editor + result grid. Rewritten from
// the upstream `routes/query.tsx`:
//   * No `createFileRoute`. SQL state is owned by `useSqlState()`
//     (localStorage-backed, mirrors the deleted `SqlProvider`).
//   * Result data goes through `useChQuery()`.
//   * Visible strings via `useExplorerMessages()`.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import "react-data-grid/lib/styles.css";
import { useState } from "react";

import { DataGrid } from "react-data-grid";
import { useDebounce } from "@uidotdev/usehooks";
import { Database, Play, ShieldX, Terminal } from "lucide-react";

import { cn } from "../lib/utils.js";
import {
  Card,
  CardTitle,
  CardHeader,
  CardDescription,
} from "../components/ui/card.js";
import { Editor } from "../components/editor.js";
import { Toggle } from "../components/ui/toggle.js";
import { Button } from "../components/ui/button.js";
import { Skeleton } from "../components/ui/skeleton.js";
import { useChQuery, useResolvedTheme, useSqlState } from "../hooks/index.js";
import { useExplorerMessages } from "../i18n/index.js";

export function ExplorerQuery() {
  const m = useExplorerMessages();
  const currentTheme = useResolvedTheme();

  const [codeState, setCodeState] = useSqlState();
  const code = useDebounce(codeState, 100);

  const [autoExecute, setAutoExecute] = useState(true);

  const { data, error, refetch } = useChQuery({
    sql: code,
    enabled: autoExecute,
  });

  const grid = !data ? (
    !autoExecute && code && error ? (
      <Card>
        <CardHeader className="flex items-center">
          <ShieldX className="mb-2 h-12 w-12 text-red-400" />
          <CardTitle className="text-red-400">{m.query.errorTitle}</CardTitle>
          <CardDescription className="text-red-400">
            {m.query.errorDescription}
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
        <CardTitle>{m.query.noResultsTitle}</CardTitle>
        <CardDescription>{m.query.noResultsDescription}</CardDescription>
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
      <div className="grid gap-4 grid-cols-1">
        <Editor value={codeState} onChange={(val) => setCodeState(val)} />

        <div className="flex gap-2 justify-between">
          <div className="flex gap-2">
            <Toggle
              size="sm"
              variant="outline"
              className="text-foreground"
              pressed={autoExecute}
              onPressedChange={(val) => setAutoExecute(val)}
              title={
                autoExecute
                  ? m.query.autoExecuteDisable
                  : m.query.autoExecuteEnable
              }
            >
              <Terminal className="h-4 w-4" />
            </Toggle>

            {!autoExecute && (
              <Button size="sm" onClick={() => refetch()}>
                <Play className="mr-2 h-4 w-4" /> {m.query.execute}
              </Button>
            )}
          </div>
        </div>
      </div>

      {grid}
    </div>
  );
}
