// Stage 1: visual flow editor wired to GET/PUT /api/flows/{id}.
//
// The canvas + node-kind registry come from `@nube/starter-ui-flow`.
// This page owns the graph as React state, feeds it to <FlowCanvas>,
// and persists via the REST client. Optimistic-lock conflicts (409)
// surface as a non-destructive banner so the user keeps their edits.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  FlowCanvas,
  NodePalette,
  NodeKindRegistry,
  BUILTIN_NODE_KINDS,
  type FlowGraph,
  type FlowNode,
  type NodeKindSpec,
  type NodeRunState,
  type RunOverlay,
} from "@nube/starter-ui-flow";
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Badge,
  Button,
} from "@nube/starter-ui-kit";

import { ApiError, api, type Flow, type Run } from "../lib/api";
import { useSse } from "../lib/sse";

// SSE payloads emitted by `GET /api/flows/{id}/events`. Mirrors the
// `RunEvent` enum in `src/sse.rs`. Kept narrow — only the variants the
// editor reduces into overlay state.
type RunEventDto =
  | { type: "run-started"; flow_id: string; run_id: string }
  | {
      type: "node-status";
      flow_id: string;
      run_id: string;
      node_id: string;
      status: string;
    }
  | {
      type: "edge-active";
      flow_id: string;
      run_id: string;
      edge_id: string;
    }
  | {
      type: "run-finished";
      flow_id: string;
      run_id: string;
      status: string;
    };

const EMPTY_OVERLAY: RunOverlay = { nodes: {}, activeEdges: [] };
const TERMINAL_CLEAR_MS = 1000;

function statusToNodeRunState(status: string): NodeRunState {
  switch (status) {
    case "running":
      return "running";
    case "ok":
    case "success":
    case "completed":
      return "ok";
    case "error":
    case "failed":
      return "error";
    case "cancelled":
      return "cancelled";
    case "skipped":
      return "skipped";
    case "ready":
      return "ready";
    default:
      return "idle";
  }
}

function runBadgeVariant(
  status: string,
): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "ok":
    case "success":
    case "completed":
      return "default";
    case "running":
      return "secondary";
    case "error":
    case "failed":
      return "destructive";
    default:
      return "outline";
  }
}

// Module-scope registry: seeded once with the built-in kinds. Future
// stages add agent-backed kinds via `registry.register(...)` here.
const nodeRegistry = new NodeKindRegistry().registerAll(BUILTIN_NODE_KINDS);

const EMPTY_GRAPH: FlowGraph = { nodes: [], edges: [] };

function makeNodeId(kind: string): string {
  // crypto.randomUUID is available in modern browsers and jsdom.
  const uuid =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : Math.random().toString(36).slice(2, 10);
  return `${kind}-${uuid.slice(0, 8)}`;
}

export function FlowEditor() {
  const { id = "" } = useParams();
  const qc = useQueryClient();

  const flowQuery = useQuery<Flow>({
    queryKey: ["flow", id],
    queryFn: () => api.flows.get(id),
    enabled: !!id,
  });

  // Local working copy of the graph. The canvas owns its own React
  // Flow state internally; this is the source of truth we save.
  const [graph, setGraph] = useState<FlowGraph>(EMPTY_GRAPH);
  const [dirty, setDirty] = useState(false);
  // Bumped whenever the graph changes from *outside* the canvas
  // (initial load, palette insert, server-reload). The bump forces
  // <FlowCanvas> to remount with the new `initial` value — its
  // internal state is otherwise only seeded once.
  const [canvasKey, setCanvasKey] = useState(0);
  // Stash the last server graph so we can offer a clean reload after
  // a 409 conflict.
  const [conflict, setConflict] = useState<Flow | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);

  // Seed local state once per loaded flow id/version.
  const loadedVersionRef = useRef<{ id: string; version: number } | null>(null);
  useEffect(() => {
    const f = flowQuery.data;
    if (!f) return;
    const last = loadedVersionRef.current;
    if (last && last.id === f.id && last.version === f.version) return;
    loadedVersionRef.current = { id: f.id, version: f.version };
    setGraph(f.graph ?? EMPTY_GRAPH);
    setDirty(false);
    setCanvasKey((k) => k + 1);
  }, [flowQuery.data]);

  const save = useMutation({
    mutationFn: async () => {
      const f = flowQuery.data;
      if (!f) throw new Error("flow not loaded");
      return api.flows.update(f.id, {
        name: f.name,
        description: f.description ?? undefined,
        graph,
        version: f.version,
      });
    },
    onSuccess: (updated) => {
      setSaveError(null);
      setConflict(null);
      setDirty(false);
      loadedVersionRef.current = { id: updated.id, version: updated.version };
      qc.setQueryData(["flow", updated.id], updated);
    },
    onError: async (err) => {
      if (err instanceof ApiError && err.status === 409) {
        // Optimistic-lock conflict — fetch the latest server state so
        // the user can review before overwriting their edits.
        try {
          const server = await api.flows.get(id);
          setConflict(server);
          setSaveError(null);
        } catch (e) {
          setSaveError(e instanceof Error ? e.message : String(e));
        }
        return;
      }
      setSaveError(err instanceof Error ? err.message : String(err));
    },
  });

  // --- Live run overlay (SSE) ---------------------------------------
  //
  // `RunEvent`s are reduced into a `RunOverlay` shape that <FlowCanvas>
  // consumes. Only events for the currently active run are kept so a
  // stale prior run can't bleed colour into the new one.
  const [overlay, setOverlay] = useState<RunOverlay>(EMPTY_OVERLAY);
  const [activeRunId, setActiveRunId] = useState<string | null>(null);
  const clearTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const runsQuery = useQuery<Run[]>({
    queryKey: ["flow-runs", id],
    queryFn: () => api.flows.runs(id),
    enabled: !!id,
    // Keep a small refresh cadence so the side panel stays in sync even
    // without a dedicated runs SSE channel.
    refetchInterval: 5_000,
  });

  const handleRunEvent = useCallback(
    (ev: RunEventDto) => {
      switch (ev.type) {
        case "run-started": {
          if (clearTimerRef.current) {
            clearTimeout(clearTimerRef.current);
            clearTimerRef.current = null;
          }
          setActiveRunId(ev.run_id);
          setOverlay({ nodes: {}, activeEdges: [] });
          // The run row appears in the panel via the polling query;
          // nudge it once so the new run shows up immediately.
          void runsQuery.refetch();
          break;
        }
        case "node-status": {
          // Drop events from prior runs.
          if (activeRunId && ev.run_id !== activeRunId) return;
          setOverlay((prev) => ({
            nodes: {
              ...prev.nodes,
              [ev.node_id]: statusToNodeRunState(ev.status),
            },
            activeEdges: prev.activeEdges ?? [],
          }));
          break;
        }
        case "edge-active": {
          if (activeRunId && ev.run_id !== activeRunId) return;
          setOverlay((prev) => {
            const existing = prev.activeEdges ?? [];
            if (existing.includes(ev.edge_id)) return prev;
            return {
              nodes: prev.nodes,
              activeEdges: [...existing, ev.edge_id],
            };
          });
          break;
        }
        case "run-finished": {
          if (activeRunId && ev.run_id !== activeRunId) return;
          // Hold the terminal frame for 1s so the user sees the final
          // colours, then clear the overlay.
          if (clearTimerRef.current) clearTimeout(clearTimerRef.current);
          clearTimerRef.current = setTimeout(() => {
            setOverlay(EMPTY_OVERLAY);
            setActiveRunId(null);
            clearTimerRef.current = null;
          }, TERMINAL_CLEAR_MS);
          void runsQuery.refetch();
          break;
        }
      }
    },
    [activeRunId, runsQuery],
  );

  useSse<RunEventDto>(id ? `/api/flows/${id}/events` : null, handleRunEvent);

  useEffect(
    () => () => {
      if (clearTimerRef.current) clearTimeout(clearTimerRef.current);
    },
    [],
  );

  const fire = useMutation({
    mutationFn: async () => api.flows.fire(id, {}),
    onSuccess: (res) => {
      // The `run-started` SSE will arrive momentarily and replace this
      // — but seeding it now means UI feedback is immediate even if the
      // event is briefly delayed.
      setActiveRunId(res.run_id);
      setOverlay({ nodes: {}, activeEdges: [] });
      void runsQuery.refetch();
    },
  });

  const palettePicks = useMemo(() => nodeRegistry.list().map((e) => e.spec), []);

  function handleCanvasChange(next: FlowGraph) {
    setGraph(next);
    setDirty(true);
  }

  function handlePalettePick(spec: NodeKindSpec) {
    const next: FlowNode = {
      id: makeNodeId(spec.kind),
      kind: spec.kind,
      // Offset each new node so they don't all stack on (0,0).
      position: {
        x: 80 + (graph.nodes.length % 6) * 60,
        y: 80 + Math.floor(graph.nodes.length / 6) * 100,
      },
      label: spec.label,
    };
    setGraph({ nodes: [...graph.nodes, next], edges: graph.edges });
    setDirty(true);
    setCanvasKey((k) => k + 1);
  }

  function discardAndReload() {
    if (!conflict) return;
    setGraph(conflict.graph ?? EMPTY_GRAPH);
    loadedVersionRef.current = { id: conflict.id, version: conflict.version };
    qc.setQueryData(["flow", conflict.id], conflict);
    setConflict(null);
    setDirty(false);
    setCanvasKey((k) => k + 1);
  }

  if (!id) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        Missing flow id.
      </div>
    );
  }

  if (flowQuery.isLoading) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        Loading flow…
      </div>
    );
  }

  if (flowQuery.isError || !flowQuery.data) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-destructive">
        Failed to load flow: {String(flowQuery.error ?? "unknown error")}
      </div>
    );
  }

  const flow = flowQuery.data;

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between border-b border-border/60 px-6 py-3">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">{flow.name}</h1>
          <p className="text-xs text-muted-foreground">
            {id} · v{flow.version}
            {dirty ? " · unsaved changes" : ""}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => flowQuery.refetch()}
            disabled={flowQuery.isFetching}
          >
            Refresh
          </Button>
          <Button
            size="sm"
            onClick={() => save.mutate()}
            disabled={!dirty || save.isPending}
          >
            {save.isPending ? "Saving…" : "Save"}
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => fire.mutate()}
            disabled={fire.isPending || dirty || activeRunId !== null}
            title={
              dirty
                ? "Save before running"
                : activeRunId !== null
                  ? "A run is already in progress"
                  : "Fire the flow"
            }
          >
            {fire.isPending
              ? "Firing…"
              : activeRunId !== null
                ? "Running…"
                : "Run"}
          </Button>
        </div>
      </header>

      {conflict ? (
        <Alert className="mx-6 mt-3 border-amber-500/60 bg-amber-500/5">
          <AlertTitle>Server has a newer version (v{conflict.version})</AlertTitle>
          <AlertDescription className="flex items-center justify-between gap-3">
            <span>
              Your edits are still in the canvas. Save again to keep working
              from the server graph, or reload to discard your changes.
            </span>
            <div className="flex gap-2">
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setConflict(null)}
              >
                Keep editing
              </Button>
              <Button size="sm" variant="outline" onClick={discardAndReload}>
                Reload server graph
              </Button>
            </div>
          </AlertDescription>
        </Alert>
      ) : null}

      {saveError ? (
        <Alert className="mx-6 mt-3 border-destructive/60 bg-destructive/5">
          <AlertTitle>Save failed</AlertTitle>
          <AlertDescription>{saveError}</AlertDescription>
        </Alert>
      ) : null}

      <div className="flex items-center gap-2 border-b border-border/60 bg-muted/30 px-4 py-2">
        <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
          Insert
        </span>
        <div className="flex flex-wrap gap-1.5">
          {palettePicks.map((spec) => (
            <button
              key={spec.kind}
              type="button"
              onClick={() => handlePalettePick(spec)}
              className="flex items-center gap-1.5 rounded-md border border-border/60 bg-background px-2.5 py-1 text-xs font-medium shadow-sm transition-colors hover:bg-accent"
            >
              <span
                className="inline-block h-2 w-2 rounded-sm"
                style={{ background: spec.color ?? "#94a3b8" }}
              />
              {spec.label}
            </button>
          ))}
        </div>
      </div>

      {fire.isError ? (
        <Alert className="mx-6 mt-3 border-destructive/60 bg-destructive/5">
          <AlertTitle>Run failed to start</AlertTitle>
          <AlertDescription>
            {fire.error instanceof Error
              ? fire.error.message
              : String(fire.error)}
          </AlertDescription>
        </Alert>
      ) : null}

      <div className="relative min-h-0 flex-1">
        <FlowCanvas
          key={canvasKey}
          registry={nodeRegistry}
          graph={graph}
          overlay={overlay}
          onChange={handleCanvasChange}
        >
          {/* Floating palette overlay (richer than the top strip — kept
              for keyboard/discovery; the top strip is the primary
              insert affordance). */}
          <div className="absolute right-3 top-3 z-10">
            <NodePalette registry={nodeRegistry} onPick={handlePalettePick} />
          </div>
        </FlowCanvas>
      </div>

      <RecentRunsPanel
        runs={runsQuery.data ?? []}
        loading={runsQuery.isLoading}
        activeRunId={activeRunId}
      />
    </div>
  );
}

// Last 10 runs for this flow. Status + relative timestamp.
function RecentRunsPanel({
  runs,
  loading,
  activeRunId,
}: {
  runs: Run[];
  loading: boolean;
  activeRunId: string | null;
}) {
  const recent = runs.slice(0, 10);
  return (
    <aside className="border-t border-border/60 bg-muted/20 px-6 py-3">
      <div className="mb-2 flex items-center justify-between">
        <h2 className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
          Recent runs
        </h2>
        <span className="text-[10px] text-muted-foreground">
          {loading ? "Loading…" : `${recent.length} shown`}
        </span>
      </div>
      {recent.length === 0 ? (
        <p className="text-xs text-muted-foreground">
          No runs yet. Hit Run to fire this flow.
        </p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {recent.map((run) => {
            const isActive = run.id === activeRunId;
            // Treat the in-flight run as "running" even if the row was
            // persisted with an earlier status snapshot.
            const status = isActive ? "running" : run.status;
            return (
              <li
                key={run.id}
                className="flex items-center justify-between gap-3 rounded-md border border-border/40 bg-background px-3 py-1.5 text-xs"
              >
                <div className="flex items-center gap-2 min-w-0">
                  <Badge variant={runBadgeVariant(status)}>{status}</Badge>
                  <code className="truncate font-mono text-[11px] text-muted-foreground">
                    {run.id.slice(0, 8)}
                  </code>
                </div>
                <span className="text-[11px] text-muted-foreground">
                  {formatTimestamp(run.finished_at ?? run.started_at)}
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </aside>
  );
}

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString();
}
