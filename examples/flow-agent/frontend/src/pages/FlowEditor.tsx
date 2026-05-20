// Stage 1: visual flow editor wired to GET/PUT /api/flows/{id}.
//
// The canvas + node-kind registry come from `@nube/starter-ui-flow`.
// This page owns the graph as React state, feeds it to <FlowCanvas>,
// and persists via the REST client. Optimistic-lock conflicts (409)
// surface as a non-destructive banner so the user keeps their edits.

import { useEffect, useMemo, useRef, useState } from "react";
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
} from "@nube/starter-ui-flow";
import { Alert, AlertDescription, AlertTitle, Button } from "@nube/starter-ui-kit";

import { ApiError, api, type Flow } from "../lib/api";

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

      <div className="relative min-h-0 flex-1">
        <FlowCanvas
          key={canvasKey}
          registry={nodeRegistry}
          graph={graph}
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
    </div>
  );
}
