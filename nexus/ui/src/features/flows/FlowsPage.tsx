import { useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  AlertTriangle,
  Bug,
  Download,
  Pause,
  Pencil,
  Play,
  Plus,
  Trash2,
  Upload,
  Workflow,
} from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import type { FlowExport, FlowSummary } from "@/api/types";
import {
  useExportFlow,
  useFlowActions,
  useFlows,
  useImportFlow,
} from "@/features/flows/useFlows";
import {
  downloadJson,
  fileStem,
  readJsonFile,
} from "@/features/flows/portabilityFile";
import { FlowBuilder } from "@/features/flows/builder/FlowBuilder";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Flow management: list the tenant's saved ingestion flows with their
// running state and start/stop + delete actions, plus a config editor to
// author new ones — all over the real endpoints. Loading/empty/error
// throughout (F0).
export function FlowsPage() {
  const { data, isPending, isError, error } = useFlows();
  const actions = useFlowActions();
  const navigate = useNavigate();
  const [creating, setCreating] = useState(false);

  // Editing and debugging both happen on the flow's own full-page route
  // (`/flows/<name>`), which is deep-linkable and gives the canvas/node config
  // the whole viewport. `?debug` opens that page with the live debug view on.
  const openFlow = (flow: FlowSummary, opts?: { debug?: boolean }) =>
    navigate(
      `/flows/${encodeURIComponent(flow.name)}${opts?.debug ? "?debug=1" : ""}`,
    );

  // Which flow id (if any) each mutation is currently acting on, so only the
  // affected row shows the busy/disabled state instead of the whole list.
  const pendingId =
    (actions.start.isPending && actions.start.variables) ||
    (actions.stop.isPending && actions.stop.variables) ||
    (actions.remove.isPending && actions.remove.variables) ||
    null;

  // The id + message of the most recent failed start/stop, so the row that the
  // user clicked shows *why* nothing happened (a 400 from an invalid config is
  // otherwise silent).
  const actionError =
    actions.start.error && actions.start.variables
      ? { id: actions.start.variables, message: actions.start.error.message }
      : actions.stop.error && actions.stop.variables
        ? { id: actions.stop.variables, message: actions.stop.error.message }
        : null;

  // Share/import. Export fetches the portable model and downloads it; import
  // reads a picked file and re-creates the flow. A banner surfaces an import
  // error or the "credentials were removed" note from an export.
  const exportFlow = useExportFlow();
  const importFlow = useImportFlow();
  const fileInput = useRef<HTMLInputElement>(null);
  const [banner, setBanner] = useState<{
    kind: "info" | "error";
    message: string;
  } | null>(null);

  const onExport = (flow: FlowSummary) => {
    setBanner(null);
    exportFlow.mutate(flow.id, {
      onSuccess: (model) => {
        downloadJson(`${fileStem(flow.name)}.flow.json`, model);
        if (model.redacted) {
          setBanner({
            kind: "info",
            message: `Credentials were removed from "${flow.name}". Whoever imports it will need to re-enter them.`,
          });
        }
      },
      onError: () =>
        setBanner({ kind: "error", message: "Couldn't export the flow." }),
    });
  };

  const onPickFile = async (file: File | undefined) => {
    if (!file) return;
    setBanner(null);
    let model: unknown;
    try {
      model = await readJsonFile(file);
    } catch (err) {
      setBanner({
        kind: "error",
        message: err instanceof Error ? err.message : "Couldn't read the file.",
      });
      return;
    }
    if (!isFlowExport(model)) {
      setBanner({
        kind: "error",
        message: "That file isn't a flow export.",
      });
      return;
    }
    importFlow.mutate(model, {
      onSuccess: (flow) =>
        setBanner({
          kind: "info",
          message: `Imported "${flow.name}". It's stopped — open it to review${
            model.redacted ? " and re-enter credentials" : ""
          }, then start it.`,
        }),
      onError: (err) =>
        setBanner({
          kind: "error",
          message: err.message || "Couldn't import the flow.",
        }),
    });
  };

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-base font-semibold tracking-tight">Flows</h2>
        <div className="flex items-center gap-2">
          {/* Hidden picker: the Import button forwards its click here. */}
          <input
            ref={fileInput}
            type="file"
            accept="application/json,.json"
            className="hidden"
            onChange={(e) => {
              void onPickFile(e.target.files?.[0]);
              // Reset so picking the same file again still fires onChange.
              e.target.value = "";
            }}
          />
          <Button
            variant="outline"
            size="sm"
            className="gap-2"
            disabled={importFlow.isPending}
            onClick={() => fileInput.current?.click()}
          >
            <Upload className="size-4" />
            {importFlow.isPending ? "Importing…" : "Import"}
          </Button>
          <Button size="sm" className="gap-2" onClick={() => setCreating(true)}>
            <Plus className="size-4" />
            New flow
          </Button>
        </div>
      </div>

      {banner ? (
        <p
          role={banner.kind === "error" ? "alert" : "status"}
          className={`flex items-center gap-2 rounded-lg border px-3 py-2 text-xs ${
            banner.kind === "error"
              ? "border-destructive/40 text-destructive"
              : "border-border/60 text-muted-foreground"
          }`}
        >
          <AlertTriangle className="size-3.5 shrink-0" aria-hidden />
          <span>{banner.message}</span>
        </p>
      ) : null}

      <div className="min-h-0 flex-1">
        {isPending ? (
          <Loading label="Loading flows…" />
        ) : isError ? (
          <ErrorState message={error instanceof Error ? error.message : undefined} />
        ) : data.length === 0 ? (
          <Empty
            title="No flows"
            description="Flows are long-running ingestion pipelines. Create one to begin."
          />
        ) : (
          <ul className="flex flex-col gap-2">
            {data.map((flow) => (
              <FlowRow
                key={flow.id}
                flow={flow}
                busy={pendingId === flow.id}
                exporting={
                  exportFlow.isPending && exportFlow.variables === flow.id
                }
                actionError={
                  actionError?.id === flow.id ? actionError.message : null
                }
                actions={actions}
                onEdit={() => openFlow(flow)}
                onExport={() => onExport(flow)}
                onDebug={() => openFlow(flow, { debug: true })}
              />
            ))}
          </ul>
        )}
      </div>

      <FlowBuilder
        open={creating}
        onOpenChange={(open) => {
          if (!open) setCreating(false);
        }}
      />
    </div>
  );
}

function FlowRow({
  flow,
  busy,
  exporting,
  actionError,
  actions,
  onEdit,
  onExport,
  onDebug,
}: {
  flow: FlowSummary;
  busy: boolean;
  exporting: boolean;
  // A start/stop error for *this* flow (e.g. a 400 from invalid config), shown
  // inline so a click that the server rejected isn't silent.
  actionError: string | null;
  actions: ReturnType<typeof useFlowActions>;
  onEdit: () => void;
  onExport: () => void;
  onDebug: () => void;
}) {
  return (
    <li className="glass flex items-center gap-3 rounded-lg px-4 py-3">
      <span className="grid size-9 place-items-center rounded-lg bg-primary/15 text-primary">
        <Workflow className="size-4" />
      </span>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-foreground">{flow.name}</p>
        <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <span
            className="size-1.5 rounded-full"
            style={{
              backgroundColor: flow.running
                ? "var(--chart-1)"
                : "var(--muted-foreground)",
              boxShadow: flow.running ? "0 0 8px var(--chart-1)" : undefined,
            }}
            aria-hidden
          />
          {flow.running ? "Running" : flow.enabled ? "Stopped" : "Disabled"}
          {flow.metrics.last_started_at && !flow.metrics.last_error ? (
            <span className="text-muted-foreground/70">
              · since {flow.metrics.last_started_at}
            </span>
          ) : null}
        </p>
        {actionError ? (
          <p
            role="alert"
            className="mt-0.5 flex items-center gap-1 truncate text-[11px] text-destructive"
            title={actionError}
          >
            <AlertTriangle className="size-3 shrink-0" aria-hidden />
            <span className="truncate">{actionError}</span>
          </p>
        ) : flow.metrics.last_error ? (
          <p
            className="mt-0.5 flex items-center gap-1 truncate text-[11px] text-destructive"
            title={flow.metrics.last_error}
          >
            <AlertTriangle className="size-3 shrink-0" aria-hidden />
            <span className="truncate">{flow.metrics.last_error}</span>
          </p>
        ) : null}
      </div>
      <Button
        variant="outline"
        size="sm"
        className="gap-2"
        disabled={busy}
        onClick={() =>
          flow.running
            ? actions.stop.mutate(flow.id)
            : actions.start.mutate(flow.id)
        }
      >
        {flow.running ? (
          <>
            <Pause className="size-4" /> Stop
          </>
        ) : (
          <>
            <Play className="size-4" /> Start
          </>
        )}
      </Button>
      {/* Debug a running flow: open the live values/per-node drawer. Only
          meaningful while the flow is running on this node. */}
      {flow.running ? (
        <Button
          variant="ghost"
          size="icon"
          aria-label={`Debug ${flow.name}`}
          title="Debug live values"
          onClick={onDebug}
          className="text-muted-foreground hover:text-foreground"
        >
          <Bug className="size-4" />
        </Button>
      ) : null}
      <Button
        variant="ghost"
        size="icon"
        aria-label={`Edit ${flow.name}`}
        // Editing a running flow would let the canvas drift from the live run;
        // stop it first.
        disabled={busy || flow.running}
        title={flow.running ? "Stop the flow to edit it" : undefined}
        onClick={onEdit}
        className="text-muted-foreground hover:text-foreground"
      >
        <Pencil className="size-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        aria-label={`Export ${flow.name}`}
        title="Export to a shareable file"
        disabled={exporting}
        onClick={onExport}
        className="text-muted-foreground hover:text-foreground"
      >
        <Download className="size-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        aria-label={`Delete ${flow.name}`}
        disabled={busy}
        onClick={() => actions.remove.mutate(flow.id)}
        className="text-muted-foreground hover:text-destructive"
      >
        <Trash2 className="size-4" />
      </Button>
    </li>
  );
}

// A light shape check on a picked file so a wrong file fails with a clear
// message instead of a 400 from the server. The backend still validates
// `schema_version` and the config; this only guards the obvious mistakes.
function isFlowExport(value: unknown): value is FlowExport {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.schema_version === "number" &&
    typeof v.name === "string" &&
    "input" in v &&
    "output" in v
  );
}
