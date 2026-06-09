import { useMemo, useState } from "react";
import { FlaskConical, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";
import { Switch } from "@nube/starter-ui-kit/components/switch";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit/components/tabs";
import { Textarea } from "@nube/starter-ui-kit/components/textarea";

import type { NodeType } from "@/api/types";
import { useCreateFlow } from "@/features/flows/useFlows";
import { Canvas } from "@/features/flows/builder/Canvas";
import { DryRunResult } from "@/features/flows/builder/DryRunResult";
import { NodeConfigForm } from "@/features/flows/builder/NodeConfigForm";
import { Palette } from "@/features/flows/builder/Palette";
import { serializeGraph, toCreateFlow } from "@/features/flows/builder/graph";
import { parseGraph } from "@/features/flows/builder/parse";
import { FLOW_TEMPLATES } from "@/features/flows/builder/templates";
import { useBuilderGraph } from "@/features/flows/builder/store";
import { useDryRun, useNodeTypes } from "@/features/flows/builder/useBuilder";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// The visual flow builder: a palette (left), a graph canvas (centre), and an
// inspector (right) with a schema-driven config form and a raw-JSON escape
// hatch that round-trips with the graph. "Test" runs a bounded dry-run of the
// current graph without saving; "Save" serialises the graph to the ArkFlow
// `{input, pipeline, output}` shape and creates the flow. Replaces the
// raw-three-textareas dialog as the primary authoring path.
export function FlowBuilder({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass flex h-[85vh] max-w-[90vw] flex-col gap-3 p-0 sm:max-w-[90vw]">
        <DialogHeader className="px-5 pt-5">
          <DialogTitle>Flow builder</DialogTitle>
          <DialogDescription>
            Drag node types onto the canvas, connect input → processor → output,
            and test the pipeline before saving.
          </DialogDescription>
        </DialogHeader>
        {open ? <BuilderBody onDone={() => onOpenChange(false)} /> : null}
      </DialogContent>
    </Dialog>
  );
}

function BuilderBody({ onDone }: { onDone: () => void }) {
  const nodeTypes = useNodeTypes();
  const create = useCreateFlow();
  const dryRun = useDryRun();
  const builder = useBuilderGraph();

  const [name, setName] = useState("");
  const [enabled, setEnabled] = useState(true);
  const [tab, setTab] = useState<"config" | "raw">("config");
  const [rawText, setRawText] = useState("");
  const [rawError, setRawError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);

  const palette = nodeTypes.data ?? [];
  const selected = builder.graph.nodes.find((n) => n.id === builder.selectedId);
  const selectedType = selected
    ? palette.find((p) => p.kind === selected.kind)
    : undefined;

  // The serialised graph drives the raw-JSON view and gates Test/Save.
  const serialised = useMemo(
    () => serializeGraph(builder.graph),
    [builder.graph],
  );

  const addFromPalette = (type: NodeType) => {
    // Stagger by node count so a click-added node doesn't land on the last.
    const x = 80 + builder.graph.nodes.length * 40;
    const y = 80 + builder.graph.nodes.length * 24;
    builder.addNode(type, { x, y });
  };

  const loadTemplate = (id: string) => {
    const tpl = FLOW_TEMPLATES.find((t) => t.id === id);
    if (!tpl) return;
    builder.setGraph(tpl.build());
    setSaveError(null);
  };

  // Sync the graph → raw text whenever the raw tab opens, so the escape hatch
  // shows the current graph; editing it and applying parses back to a graph.
  const openRaw = () => {
    if (serialised.ok) {
      setRawText(
        JSON.stringify(
          {
            input: serialised.input,
            pipeline: serialised.pipeline,
            output: serialised.output,
          },
          null,
          2,
        ),
      );
      setRawError(null);
    } else {
      setRawError(serialised.error);
    }
    setTab("raw");
  };

  const applyRaw = () => {
    let parsed: unknown;
    try {
      parsed = JSON.parse(rawText);
    } catch (err) {
      setRawError(err instanceof Error ? err.message : "Invalid JSON");
      return;
    }
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
      setRawError("Expected an object with input / pipeline / output.");
      return;
    }
    const obj = parsed as Record<string, unknown>;
    const graph = parseGraph(obj.input, obj.pipeline, obj.output, palette);
    if (graph.nodes.length === 0) {
      setRawError("Couldn't read any nodes from that JSON.");
      return;
    }
    builder.setGraph(graph);
    setRawError(null);
    setTab("config");
  };

  const runTest = () => {
    if (!serialised.ok) return;
    setSaveError(null);
    dryRun.mutate({ input: serialised.input, pipeline: serialised.pipeline });
  };

  const save = () => {
    setSaveError(null);
    const built = toCreateFlow(builder.graph, name, enabled);
    if (!built.ok) {
      setSaveError(built.error);
      return;
    }
    if (!built.value.name) {
      setSaveError("Give the flow a name.");
      return;
    }
    create.mutate(built.value, {
      onSuccess: onDone,
      onError: () => setSaveError("Couldn't save the flow."),
    });
  };

  if (nodeTypes.isPending) {
    return (
      <div className="flex-1 px-5 pb-5">
        <Loading label="Loading node types…" />
      </div>
    );
  }
  if (nodeTypes.isError) {
    return (
      <div className="flex-1 px-5 pb-5">
        <ErrorState
          message={
            nodeTypes.error instanceof Error
              ? nodeTypes.error.message
              : undefined
          }
        />
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 px-5 pb-5">
      <div className="flex flex-wrap items-end gap-3">
        <div className="min-w-48 flex-1 space-y-1.5">
          <Label htmlFor="builder-name">Name</Label>
          <Input
            id="builder-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="weather → timescale"
          />
        </div>
        <div className="w-56 space-y-1.5">
          <Label htmlFor="builder-template">Start from a template</Label>
          <Select onValueChange={loadTemplate}>
            <SelectTrigger id="builder-template">
              <SelectValue placeholder="Choose a template…" />
            </SelectTrigger>
            <SelectContent>
              {FLOW_TEMPLATES.map((t) => (
                <SelectItem key={t.id} value={t.id}>
                  {t.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <label className="flex items-center gap-2 pb-2 text-sm">
          <Switch checked={enabled} onCheckedChange={setEnabled} />
          Enabled
        </label>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-[200px_1fr_320px] gap-3">
        <div className="scrollbar-thin min-h-0 overflow-auto rounded-lg border border-border/60 p-2.5">
          <Palette nodeTypes={palette} onAdd={addFromPalette} />
        </div>

        <div className="min-h-0 overflow-hidden rounded-lg border border-border/60">
          {builder.graph.nodes.length === 0 ? (
            <div className="grid h-full place-items-center p-6 text-center text-sm text-muted-foreground">
              Click a node type on the left to add it, or load a template.
            </div>
          ) : (
            <Canvas builder={builder} />
          )}
        </div>

        <div className="scrollbar-thin flex min-h-0 flex-col overflow-auto rounded-lg border border-border/60 p-3">
          <Tabs value={tab} onValueChange={(v) => (v === "raw" ? openRaw() : setTab("config"))}>
            <TabsList className="w-full">
              <TabsTrigger value="config" className="flex-1">
                Config
              </TabsTrigger>
              <TabsTrigger value="raw" className="flex-1">
                Raw JSON
              </TabsTrigger>
            </TabsList>
            <TabsContent value="config" className="pt-3">
              {selected ? (
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <p className="text-sm font-medium text-foreground">
                      {selectedType?.label ?? selected.kind}
                    </p>
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label="Remove node"
                      className="text-muted-foreground hover:text-destructive"
                      onClick={() => builder.removeNode(selected.id)}
                    >
                      <Trash2 className="size-4" />
                    </Button>
                  </div>
                  <NodeConfigForm
                    schema={selectedType?.config_schema}
                    config={selected.config}
                    onChange={(config) => builder.setConfig(selected.id, config)}
                  />
                </div>
              ) : (
                <p className="text-xs text-muted-foreground">
                  Select a node to configure it.
                </p>
              )}
            </TabsContent>
            <TabsContent value="raw" className="space-y-2 pt-3">
              <Textarea
                value={rawText}
                onChange={(e) => setRawText(e.target.value)}
                spellCheck={false}
                className="min-h-64 resize-y font-mono text-xs"
                aria-label="Flow config as JSON"
              />
              {rawError ? (
                <p role="alert" className="text-xs text-destructive">
                  {rawError}
                </p>
              ) : null}
              <Button variant="outline" size="sm" onClick={applyRaw}>
                Apply to canvas
              </Button>
            </TabsContent>
          </Tabs>
        </div>
      </div>

      {dryRun.data || dryRun.isPending || dryRun.isError ? (
        <div className="max-h-48 min-h-0 overflow-auto rounded-lg border border-border/60 p-3">
          {dryRun.isPending ? (
            <Loading label="Running sample…" />
          ) : dryRun.isError ? (
            <ErrorState
              message={
                dryRun.error instanceof Error ? dryRun.error.message : undefined
              }
            />
          ) : dryRun.data ? (
            <DryRunResult result={dryRun.data} />
          ) : null}
        </div>
      ) : null}

      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0 text-xs text-destructive">
          {!serialised.ok && builder.graph.nodes.length > 0 ? serialised.error : null}
          {saveError ? <span role="alert">{saveError}</span> : null}
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            className="gap-2"
            disabled={!serialised.ok || dryRun.isPending}
            onClick={runTest}
          >
            <FlaskConical className="size-4" />
            {dryRun.isPending ? "Testing…" : "Test"}
          </Button>
          <Button
            disabled={!serialised.ok || create.isPending || !name.trim()}
            onClick={save}
          >
            {create.isPending ? "Saving…" : "Save flow"}
          </Button>
        </div>
      </div>
    </div>
  );
}
