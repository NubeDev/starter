import { useMemo, useState } from "react";
import { AlertTriangle, Bug, Play, Radio } from "lucide-react";
import { useMutation } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Textarea } from "@nube/starter-ui-kit/components/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";
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

import { useFlow } from "@/features/flows/useFlows";
import { useNodeTypes } from "@/features/flows/builder/useBuilder";
import { parseGraph } from "@/features/flows/builder/parse";
import { DebugCanvas } from "@/features/flows/DebugCanvas";
import {
  useFlowDebug,
  type DebugLogLine,
  type NodeDebug,
} from "@/features/flows/useFlowDebug";
import { queryFlowTable } from "@/api/flows/table";
import type { QueryResponse } from "@/api/types";
import { Loading } from "@/features/state/Loading";

// Live debug & values for a running flow. Opening the drawer enables per-node
// capture server-side and subscribes to its SSE stream; closing disables it.
// The flow is shown two ways at once (the requested table + canvas): the
// read-only canvas overlays live counters on each node, and the tabbed panel
// below gives a per-node counters table, the sampled row values for the
// selected node, and the run's log lines.
export function DebugDrawer({
  flowId,
  flowName,
  open,
  onOpenChange,
}: {
  flowId: string | null;
  flowName: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass flex h-[85vh] max-w-[90vw] flex-col gap-3 p-4 sm:max-w-[90vw]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Bug className="size-4" />
            Debug · {flowName}
          </DialogTitle>
          <DialogDescription>
            Live values flowing through each node of the running flow. Capture
            is on while this is open.
          </DialogDescription>
        </DialogHeader>
        {open && flowId ? <DebugBody flowId={flowId} /> : null}
      </DialogContent>
    </Dialog>
  );
}

function DebugBody({ flowId }: { flowId: string }) {
  const flow = useFlow(flowId);
  const nodeTypes = useNodeTypes();
  const debug = useFlowDebug(flowId, true);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(0);

  const graph = useMemo(() => {
    if (!flow.data || !nodeTypes.data) return null;
    return parseGraph(
      flow.data.input,
      flow.data.pipeline,
      flow.data.output,
      nodeTypes.data,
    );
  }, [flow.data, nodeTypes.data]);

  if (flow.isPending || nodeTypes.isPending) {
    return <Loading label="Loading flow…" />;
  }
  if (flow.isError || !graph) {
    return (
      <p className="text-sm text-destructive">Couldn't load the flow config.</p>
    );
  }

  // The sink table this flow writes to, if any — drives the Table tab. Only
  // postgres/datasource sinks store rows; other sinks have no table to query.
  const sinkTable = sinkTableName(flow.data.output);

  const selectedNode =
    selectedIndex !== null ? debug.byNode.get(selectedIndex) : undefined;
  // Order nodes by index for the table and node labels for the values tab.
  const orderedNodes = graph.nodes.map((n, i) => ({
    index: i,
    label: n.kind,
    category: n.category,
    debug: debug.byNode.get(i),
  }));

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <ConnectionBar connection={debug.connection} error={debug.error} />

      <div className="min-h-0 flex-1 overflow-hidden rounded-lg border border-border/60">
        <DebugCanvas
          graph={graph}
          byNode={debug.byNode}
          selectedIndex={selectedIndex}
          onSelect={setSelectedIndex}
        />
      </div>

      <Tabs defaultValue="nodes" className="flex h-64 min-h-0 flex-col">
        <TabsList>
          <TabsTrigger value="nodes">Per-node</TabsTrigger>
          <TabsTrigger value="values">
            Values
            {selectedNode ? ` · ${orderedNodes[selectedIndex ?? 0]?.label}` : ""}
          </TabsTrigger>
          {sinkTable ? <TabsTrigger value="table">Table</TabsTrigger> : null}
          <TabsTrigger value="logs">
            Logs{debug.logs.length ? ` (${debug.logs.length})` : ""}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="nodes" className="min-h-0 flex-1">
          <NodeTable
            nodes={orderedNodes}
            selectedIndex={selectedIndex}
            onSelect={setSelectedIndex}
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
  connection: ReturnType<typeof useFlowDebug>["connection"];
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
  // Union of keys across the sampled rows gives stable columns even when rows
  // are sparsely populated.
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
                <TableCell key={col} className="max-w-48 truncate font-mono text-xs">
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

// Query the flow's sink table without leaving the drawer — the flow scopes the
// connection + table, so there's no datasource to pick or table name to retype.
// `{table}` expands server-side to the flow's table; the default is recent rows.
function TableTab({ flowId, table }: { flowId: string; table: string }) {
  const client = useStarterClient();
  const defaultSql = `SELECT * FROM {table} ORDER BY 1 DESC LIMIT 50`;
  const [sql, setSql] = useState(defaultSql);

  const run = useMutation<QueryResponse, Error>({
    mutationFn: () => queryFlowTable(client, flowId, { sql }),
  });

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <div className="flex items-start gap-2">
        <Textarea
          value={sql}
          onChange={(e) => setSql(e.target.value)}
          spellCheck={false}
          className="h-16 flex-1 resize-none font-mono text-xs"
          aria-label={`SQL over ${table}`}
        />
        <Button
          size="sm"
          onClick={() => run.mutate()}
          disabled={run.isPending}
          className="shrink-0"
        >
          <Play className="size-3.5" aria-hidden />
          {run.isPending ? "Running…" : "Run"}
        </Button>
      </div>
      <p className="text-[11px] text-muted-foreground">
        Read-only, scoped to <span className="font-mono">{table}</span> ·{" "}
        <span className="font-mono">{"{table}"}</span> expands to the flow's
        sink table.
      </p>
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
            Run the query to see what landed in the table.
          </p>
        )}
      </div>
    </div>
  );
}

// Render a QueryResponse (columns + rows) — shared by the Table tab and, later,
// the Transform tab's dry-run output.
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
