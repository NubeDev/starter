import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Play, Radio } from "lucide-react";
import { useMutation } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Textarea } from "@nube/starter-ui-kit/components/textarea";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit/components/tabs";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@nube/starter-ui-kit/components/table";
import { Badge } from "@nube/starter-ui-kit/components/badge";
import { ScrollArea } from "@nube/starter-ui-kit/components/scroll-area";

import { useUpdateFlow } from "@/features/flows/useFlows";
import { useDryRun } from "@/features/flows/builder/useBuilder";
import { DryRunResult } from "@/features/flows/builder/DryRunResult";
import type { FlowGraph } from "@/features/flows/builder/graph";
import type {
  DebugLogLine,
  FlowDebugState,
  NodeDebug,
} from "@/features/flows/useFlowDebug";
import { queryFlowTable } from "@/api/flows/table";
import type { QueryResponse } from "@/api/types";

// The live-debug dock that sits *below the shared canvas* in the flow editor —
// never a separate view. The canvas (with its overlaid counters) and the
// right-hand node-config panel stay visible; this dock just adds the per-node
// table, sampled values for the selected node, the sink Table query, a pipeline
// Transform sandbox, and the run log. `selectedIndex` is the same selection the
// canvas/config panel use, so clicking a node updates everything at once.
export function DebugDock({
  flowId,
  graph,
  debug,
  input,
  pipeline,
  output,
  selectedIndex,
  onSelect,
}: {
  flowId: string;
  graph: FlowGraph;
  debug: FlowDebugState;
  input: unknown;
  pipeline: unknown;
  output: unknown;
  selectedIndex: number | null;
  onSelect: (i: number) => void;
}) {
  const [tab, setTab] = useState("nodes");

  const sinkTable = sinkTableName(output);
  const selectedNode =
    selectedIndex !== null ? debug.byNode.get(selectedIndex) : undefined;
  const selectedLabel =
    selectedIndex !== null ? graph.nodes[selectedIndex]?.kind : undefined;

  const orderedNodes = graph.nodes.map((n, i) => ({
    index: i,
    label: n.kind,
    category: n.category,
    debug: debug.byNode.get(i),
  }));

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <ConnectionBar connection={debug.connection} error={debug.error} />
      <Tabs
        value={tab}
        onValueChange={setTab}
        className="flex min-h-0 flex-1 flex-col"
      >
        <TabsList>
          <TabsTrigger value="nodes">Per-node</TabsTrigger>
          <TabsTrigger value="values">
            Values{selectedLabel ? ` · ${selectedLabel}` : ""}
          </TabsTrigger>
          {sinkTable ? <TabsTrigger value="table">Table</TabsTrigger> : null}
          <TabsTrigger value="transform">Transform</TabsTrigger>
          <TabsTrigger value="logs">
            Logs{debug.logs.length ? ` (${debug.logs.length})` : ""}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="nodes" className="min-h-0 flex-1">
          <NodeTable
            nodes={orderedNodes}
            selectedIndex={selectedIndex}
            onSelect={onSelect}
          />
        </TabsContent>
        <TabsContent value="values" className="min-h-0 flex-1">
          <ValuesTable node={selectedNode} />
        </TabsContent>
        {sinkTable ? (
          <TabsContent value="table" className="min-h-0 flex-1">
            <TableTab flowId={flowId} table={sinkTable} />
          </TabsContent>
        ) : null}
        <TabsContent value="transform" className="min-h-0 flex-1">
          <TransformTab flowId={flowId} input={input} pipeline={pipeline} />
        </TabsContent>
        <TabsContent value="logs" className="min-h-0 flex-1">
          <LogList logs={debug.logs} />
        </TabsContent>
      </Tabs>
    </div>
  );
}

function ConnectionBar({
  connection,
  error,
}: {
  connection: FlowDebugState["connection"];
  error?: string;
}) {
  if (connection === "error") {
    return (
      <p
        role="alert"
        className="flex items-center gap-2 rounded-lg border border-destructive/40 px-3 py-2 text-xs text-destructive"
      >
        <AlertTriangle className="size-3.5 shrink-0" aria-hidden />
        {error ?? "Debug stream disconnected."}
      </p>
    );
  }
  const live = connection === "live";
  return (
    <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
      <Radio
        className="size-3.5"
        style={{ color: live ? "var(--chart-1)" : undefined }}
        aria-hidden
      />
      {live ? "Live" : "Connecting…"}
    </p>
  );
}

type OrderedNode = {
  index: number;
  label: string;
  category: string;
  debug?: NodeDebug;
};

function NodeTable({
  nodes,
  selectedIndex,
  onSelect,
}: {
  nodes: OrderedNode[];
  selectedIndex: number | null;
  onSelect: (i: number) => void;
}) {
  return (
    <ScrollArea className="h-full">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-10">#</TableHead>
            <TableHead>Node</TableHead>
            <TableHead className="text-right">Rows in</TableHead>
            <TableHead className="text-right">Rows out</TableHead>
            <TableHead className="text-right">Batches</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {nodes.map((n) => {
            const c = n.debug?.counters;
            return (
              <TableRow
                key={n.index}
                data-state={n.index === selectedIndex ? "selected" : undefined}
                className="cursor-pointer"
                onClick={() => onSelect(n.index)}
              >
                <TableCell className="text-muted-foreground">{n.index}</TableCell>
                <TableCell>
                  <span className="font-medium">{n.label}</span>{" "}
                  <Badge variant="outline" className="ml-1 text-[10px]">
                    {n.category}
                  </Badge>
                </TableCell>
                <TableCell className="text-right font-mono">
                  {c ? c.rows_in : "—"}
                </TableCell>
                <TableCell className="text-right font-mono">
                  {c ? c.rows_out : "—"}
                </TableCell>
                <TableCell className="text-right font-mono">
                  {c ? c.batches : "—"}
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </ScrollArea>
  );
}

function ValuesTable({ node }: { node?: NodeDebug }) {
  if (!node || node.rows.length === 0) {
    return (
      <p className="p-3 text-xs text-muted-foreground">
        No sampled rows yet for this node. Values appear as batches flow through.
      </p>
    );
  }
  const columns = Array.from(
    node.rows.reduce<Set<string>>((set, row) => {
      Object.keys(row).forEach((k) => set.add(k));
      return set;
    }, new Set()),
  );
  return (
    <ScrollArea className="h-full">
      <Table>
        <TableHeader>
          <TableRow>
            {columns.map((col) => (
              <TableHead key={col}>{col}</TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {node.rows.map((row, i) => (
            <TableRow key={i}>
              {columns.map((col) => (
                <TableCell
                  key={col}
                  className="max-w-48 truncate font-mono text-xs"
                >
                  {renderCell(row[col])}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </ScrollArea>
  );
}

function LogList({ logs }: { logs: DebugLogLine[] }) {
  if (logs.length === 0) {
    return (
      <p className="p-3 text-xs text-muted-foreground">
        No log lines yet. Retries, drops, dead-letters, and the run-ending error
        appear here.
      </p>
    );
  }
  return (
    <ScrollArea className="h-full">
      <ul className="flex flex-col gap-1 p-1 font-mono text-xs">
        {logs.map((line) => (
          <li key={line.seq} className="flex items-start gap-2">
            <span className={levelClass(line.level)}>{line.level}</span>
            {line.nodeIndex !== undefined ? (
              <span className="text-muted-foreground">[node {line.nodeIndex}]</span>
            ) : null}
            <span className="text-foreground">{line.message}</span>
          </li>
        ))}
      </ul>
    </ScrollArea>
  );
}

function levelClass(level: DebugLogLine["level"]): string {
  switch (level) {
    case "error":
      return "text-destructive shrink-0 uppercase";
    case "warn":
      return "text-amber-500 shrink-0 uppercase";
    default:
      return "text-muted-foreground shrink-0 uppercase";
  }
}

// The table a flow's sink writes to, or null when the sink stores no rows
// (sse / drop / broadcast). Mirrors the backend's sink_table_target: only
// postgres/datasource sinks carry a `table`.
function sinkTableName(output: unknown): string | null {
  if (!output || typeof output !== "object") return null;
  const o = output as Record<string, unknown>;
  const kind = typeof o.type === "string" ? o.type : "";
  if (kind !== "postgres" && kind !== "datasource") return null;
  return typeof o.table === "string" ? o.table : null;
}

// Query the flow's sink table without leaving the editor — the flow scopes the
// connection + table, so there's no datasource to pick or table name to retype.
function TableTab({ flowId, table }: { flowId: string; table: string }) {
  const client = useStarterClient();
  const [sql, setSql] = useState("");
  const [ready, setReady] = useState(false);

  const probe = useMutation<QueryResponse, Error>({
    mutationFn: () =>
      queryFlowTable(client, flowId, { sql: "SELECT * FROM {table}", limit: 1 }),
  });
  const run = useMutation<QueryResponse, Error, string>({
    mutationFn: (q: string) => queryFlowTable(client, flowId, { sql: q }),
  });

  useEffect(() => {
    if (ready) return;
    probe.mutate(undefined, {
      onSuccess: (res) => {
        const cols = res.columns ?? [];
        const tsCol = cols.find((c) => c.type === "timestamp")?.name;
        const order = tsCol ? ` ORDER BY "${tsCol}" DESC` : "";
        const def = `SELECT * FROM {table}${order} LIMIT 50`;
        setSql(def);
        setReady(true);
        run.mutate(def);
      },
      onError: () => {
        const def = `SELECT * FROM {table} LIMIT 50`;
        setSql(def);
        setReady(true);
      },
    });
    // run/probe are stable mutation handles; intentionally run once.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready]);

  const columns = probe.data?.columns ?? [];

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <div className="flex items-start gap-2">
        <Textarea
          value={sql}
          onChange={(e) => setSql(e.target.value)}
          spellCheck={false}
          className="h-16 flex-1 resize-none font-mono text-xs"
          aria-label={`SQL over ${table}`}
          placeholder={ready ? undefined : "Loading the table…"}
        />
        <Button
          size="sm"
          onClick={() => run.mutate(sql)}
          disabled={run.isPending || !sql.trim()}
          className="shrink-0"
        >
          <Play className="size-3.5" aria-hidden />
          {run.isPending ? "Running…" : "Run"}
        </Button>
      </div>

      {columns.length > 0 ? (
        <p className="flex flex-wrap gap-1 text-[11px] text-muted-foreground">
          <span className="font-mono">{table}</span>
          <span>·</span>
          {columns.map((c) => (
            <span key={c.name} className="font-mono">
              {c.name}
              <span className="text-muted-foreground/60">:{c.type}</span>
            </span>
          ))}
        </p>
      ) : (
        <p className="text-[11px] text-muted-foreground">
          Read-only, scoped to <span className="font-mono">{table}</span> ·{" "}
          <span className="font-mono">{"{table}"}</span> expands to the sink table.
        </p>
      )}

      {run.isError ? (
        <p role="alert" className="text-xs text-destructive">
          {run.error.message}
        </p>
      ) : null}
      <div className="min-h-0 flex-1">
        {run.data ? (
          <ResultTable result={run.data} />
        ) : (
          <p className="p-3 text-xs text-muted-foreground">
            {ready ? "Run the query to see rows." : "Loading the table…"}
          </p>
        )}
      </div>
    </div>
  );
}

// Edit the pipeline and dry-run it: input rows → transformed output, against the
// flow's real input connector, with NO write to the sink and NO change to the
// running flow. "Apply to flow" is an explicit, separate step that saves it.
function TransformTab({
  flowId,
  input,
  pipeline,
}: {
  flowId: string;
  input: unknown;
  pipeline: unknown;
}) {
  const initial = useMemo(
    () => JSON.stringify(pipeline ?? [], null, 2),
    [pipeline],
  );
  const [text, setText] = useState(initial);
  const dryRun = useDryRun();
  const update = useUpdateFlow();

  const parsed = useMemo(() => {
    try {
      return { value: JSON.parse(text) as unknown, error: null as string | null };
    } catch (e) {
      return { value: null, error: e instanceof Error ? e.message : "invalid JSON" };
    }
  }, [text]);

  const dirty = text.trim() !== initial.trim();

  return (
    <div className="flex h-full min-h-0 gap-3">
      <div className="flex w-1/2 min-w-0 flex-col gap-2">
        <Textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          spellCheck={false}
          className="min-h-0 flex-1 resize-none font-mono text-xs"
          aria-label="Pipeline JSON"
        />
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            onClick={() =>
              parsed.value !== null &&
              dryRun.mutate({ input: input as never, pipeline: parsed.value as never })
            }
            disabled={!!parsed.error || dryRun.isPending}
          >
            <Play className="size-3.5" aria-hidden />
            {dryRun.isPending ? "Running…" : "Dry-run"}
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() =>
              parsed.value !== null &&
              update.mutate({ id: flowId, body: { pipeline: parsed.value as never } })
            }
            disabled={!!parsed.error || !dirty || update.isPending}
            title={!dirty ? "No pipeline changes to apply" : "Save the edited pipeline to the flow"}
          >
            {update.isPending ? "Applying…" : "Apply to flow"}
          </Button>
          {parsed.error ? (
            <span className="text-xs text-destructive">{parsed.error}</span>
          ) : (
            <span className="text-[11px] text-muted-foreground">
              Dry-run is read-only: no DB write, running flow untouched.
            </span>
          )}
        </div>
        {update.isSuccess && !dirty ? (
          <p className="text-[11px] text-[color:var(--chart-1)]">
            Pipeline saved. Restart the flow for it to take effect on the live run.
          </p>
        ) : null}
      </div>
      <div className="min-h-0 w-1/2 overflow-hidden">
        {dryRun.data ? (
          <DryRunResult result={dryRun.data} />
        ) : dryRun.isError ? (
          <p role="alert" className="p-3 text-xs text-destructive">
            {dryRun.error.message}
          </p>
        ) : (
          <p className="p-3 text-xs text-muted-foreground">
            Dry-run to see the transformed output for the current pipeline.
          </p>
        )}
      </div>
    </div>
  );
}

function ResultTable({ result }: { result: QueryResponse }) {
  const columns = result.columns.map((c) => c.name);
  if (result.rows.length === 0) {
    return <p className="p-3 text-xs text-muted-foreground">No rows.</p>;
  }
  return (
    <ScrollArea className="h-full">
      <Table>
        <TableHeader>
          <TableRow>
            {columns.map((col) => (
              <TableHead key={col}>{col}</TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {result.rows.map((row, i) => (
            <TableRow key={i}>
              {columns.map((col) => (
                <TableCell
                  key={col}
                  className="max-w-48 truncate font-mono text-xs"
                >
                  {renderCell((row as Record<string, unknown>)[col])}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </ScrollArea>
  );
}

function renderCell(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}
