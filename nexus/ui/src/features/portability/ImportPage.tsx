import { useMemo, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, FileUp, Upload } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Textarea } from "@nube/starter-ui-kit/components/textarea";

import type { DashboardExport } from "@/api/types";
import { useImportDashboard } from "@/features/dashboards/useDashboardPortability";
import {
  parseExport,
  selectionCount,
  selectionTotals,
} from "@/features/portability/model";
import { PlacementPreview } from "@/features/portability/PlacementPreview";
import { VariableSelectList } from "@/features/portability/VariableSelectList";
import { useSelection } from "@/features/portability/useSelection";
import { useImportIntoDashboard } from "@/features/portability/useImportSelection";
import { readFileAsText } from "@/features/portability/fileOps";

// Where the imported items should land.
type Target = "this" | "new";

// A dedicated, full-page import flow. The user supplies an export (a file or
// pasted JSON), sees the widgets laid out exactly as they'll arrive, ticks which
// widgets/variables to take, and imports them either into the current dashboard
// or as a brand-new one. The preview is the same schematic board the export page
// shows, so the two flows feel like one tool.
export function ImportPage() {
  const { slug = "" } = useParams();
  const navigate = useNavigate();
  const [model, setModel] = useState<DashboardExport | undefined>();
  const [parseError, setParseError] = useState<string | null>(null);

  function ingest(text: string) {
    const result = parseExport(text);
    if (result.ok) {
      setModel(result.model);
      setParseError(null);
    } else {
      setModel(undefined);
      setParseError(result.error);
    }
  }

  return (
    <div className="flex h-full flex-col gap-4">
      <header className="flex items-center gap-3">
        <Button asChild variant="ghost" size="sm" className="gap-2">
          <Link to={slug ? `/d/${slug}` : "/dashboards"}>
            <ArrowLeft className="size-4" />
            Back
          </Link>
        </Button>
        <div>
          <h1 className="text-base font-semibold tracking-tight">
            Import widgets &amp; variables
          </h1>
          <p className="text-xs text-muted-foreground">
            Load an export, choose what to bring in, and place it.
          </p>
        </div>
      </header>

      {model ? (
        <ImportPreview
          model={model}
          slug={slug}
          onReplace={() => {
            setModel(undefined);
            setParseError(null);
          }}
          onDone={(targetSlug) => navigate(`/d/${targetSlug}`)}
        />
      ) : (
        <SourcePicker error={parseError} onText={ingest} />
      )}
    </div>
  );
}

// Step 1: get the JSON in — drop/select a file, or paste.
function SourcePicker({
  error,
  onText,
}: {
  error: string | null;
  onText: (text: string) => void;
}) {
  const fileInput = useRef<HTMLInputElement>(null);
  const [pasted, setPasted] = useState("");

  return (
    <div className="grid min-h-0 flex-1 gap-4 lg:grid-cols-2">
      <button
        type="button"
        onClick={() => fileInput.current?.click()}
        className="flex flex-col items-center justify-center gap-3 rounded-xl border-2 border-dashed border-border p-8 text-center transition hover:border-primary/50 hover:bg-muted/30"
      >
        <FileUp className="size-8 text-muted-foreground" />
        <span className="text-sm font-medium">Choose an export file</span>
        <span className="text-xs text-muted-foreground">
          A <code>.dashboard.json</code> file you exported earlier.
        </span>
        <input
          ref={fileInput}
          type="file"
          accept="application/json,.json"
          className="hidden"
          onChange={async (e) => {
            const file = e.target.files?.[0];
            if (file) onText(await readFileAsText(file));
            // Reset so re-picking the same file fires change again.
            e.target.value = "";
          }}
        />
      </button>

      <div className="flex min-h-0 flex-col gap-2">
        <label className="text-sm font-medium" htmlFor="import-json">
          …or paste JSON
        </label>
        <Textarea
          id="import-json"
          value={pasted}
          onChange={(e) => setPasted(e.target.value)}
          placeholder='{ "schema_version": 1, "name": "…", "panels": [ … ] }'
          className="min-h-48 flex-1 font-mono text-xs"
        />
        <Button
          variant="outline"
          className="gap-2 self-start"
          disabled={!pasted.trim()}
          onClick={() => onText(pasted)}
        >
          <Upload className="size-4" />
          Load pasted JSON
        </Button>
        {error ? (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        ) : null}
      </div>
    </div>
  );
}

// Step 2: preview + select + place + import.
function ImportPreview({
  model,
  slug,
  onReplace,
  onDone,
}: {
  model: DashboardExport;
  slug: string;
  onReplace: () => void;
  onDone: (targetSlug: string) => void;
}) {
  const { selection, togglePanel, toggleVariable, all, none } =
    useSelection(model);
  const [target, setTarget] = useState<Target>(slug ? "this" : "new");

  const importNew = useImportDashboard();
  const importHere = useImportIntoDashboard(slug);

  const totals = selectionTotals(model);
  const counts = selectionCount(selection);
  const disabled = counts.total === 0;
  const busy = importNew.isPending || importHere.isPending;

  // A report from an "add to this dashboard" run (partial-success aware).
  const report = importHere.data;

  const summary = useMemo(() => {
    if (target === "new") {
      return `Create a new dashboard from ${counts.panels} widget${
        counts.panels === 1 ? "" : "s"
      } and ${counts.variables} variable${counts.variables === 1 ? "" : "s"}.`;
    }
    return `Add ${counts.panels} widget${counts.panels === 1 ? "" : "s"} and ${
      counts.variables
    } variable${counts.variables === 1 ? "" : "s"} to this dashboard.`;
  }, [target, counts]);

  async function runImport() {
    if (target === "new") {
      // A new dashboard takes the whole (filtered) model; reuse the existing
      // import endpoint, narrowed to the selection.
      const filtered = {
        ...model,
        panels: model.panels.filter((_, i) => selection.panelIndices.has(i)),
        variables: (model.variables ?? []).filter((v) =>
          selection.variableNames.has(v.name),
        ),
      };
      const created = await importNew.mutateAsync(filtered);
      onDone(created.slug);
    } else {
      await importHere.mutateAsync({ model, selection });
      // Stay on the report (partial failures need to be seen) but refresh the
      // canvas underneath; navigation happens via the explicit button below.
    }
  }

  return (
    <div className="grid min-h-0 flex-1 gap-4 lg:grid-cols-[1fr_22rem]">
      <section className="flex min-h-0 flex-col gap-3 overflow-y-auto">
        <div className="flex items-center justify-between gap-2">
          <h3 className="text-sm font-semibold">
            Widgets{" "}
            <span className="font-normal text-muted-foreground">
              {counts.panels} of {totals.panels}
            </span>
          </h3>
          <div className="flex gap-1">
            <Button variant="ghost" size="sm" onClick={all}>
              Select all
            </Button>
            <Button variant="ghost" size="sm" onClick={none}>
              Clear
            </Button>
            <Button variant="ghost" size="sm" onClick={onReplace}>
              Use another file
            </Button>
          </div>
        </div>
        <PlacementPreview
          panels={model.panels}
          selection={selection}
          onToggle={togglePanel}
        />
      </section>

      <aside className="flex min-h-0 flex-col gap-4 overflow-y-auto">
        <section className="flex flex-col gap-2">
          <h3 className="text-sm font-semibold">
            Variables{" "}
            <span className="font-normal text-muted-foreground">
              {counts.variables} of {totals.variables}
            </span>
          </h3>
          <VariableSelectList
            variables={model.variables ?? []}
            selection={selection}
            onToggle={toggleVariable}
          />
        </section>

        <section className="mt-auto flex flex-col gap-3 rounded-xl border border-border bg-card p-3">
          {/* Target picker — only offer "this dashboard" when we're scoped to
              one (the route carried a slug). */}
          {slug ? (
            <div className="flex gap-1 rounded-lg bg-muted p-1">
              <TargetTab
                active={target === "this"}
                onClick={() => setTarget("this")}
                label="This dashboard"
              />
              <TargetTab
                active={target === "new"}
                onClick={() => setTarget("new")}
                label="New dashboard"
              />
            </div>
          ) : null}

          <p className="text-xs text-muted-foreground">
            {disabled ? "Select at least one item to import." : summary}
          </p>

          {report ? (
            <ImportResult report={report} onOpen={() => onDone(slug)} />
          ) : (
            <Button
              className="gap-2"
              disabled={disabled || busy}
              onClick={runImport}
            >
              <Upload className="size-4" />
              {busy
                ? "Importing…"
                : target === "new"
                  ? "Create dashboard"
                  : "Import here"}
            </Button>
          )}

          {(importNew.isError || importHere.isError) && !report ? (
            <p role="alert" className="text-sm text-destructive">
              {(importNew.error ?? importHere.error) instanceof Error
                ? (importNew.error ?? importHere.error)!.message
                : "Import failed."}
            </p>
          ) : null}
        </section>
      </aside>
    </div>
  );
}

function TargetTab({
  active,
  onClick,
  label,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={[
        "flex-1 rounded-md px-2 py-1 text-xs font-medium transition",
        active
          ? "bg-background text-foreground shadow-sm"
          : "text-muted-foreground hover:text-foreground",
      ].join(" ")}
    >
      {label}
    </button>
  );
}

// The partial-success report for an "add to this dashboard" import.
function ImportResult({
  report,
  onOpen,
}: {
  report: { panelsAdded: number; variablesAdded: number; failures: string[] };
  onOpen: () => void;
}) {
  return (
    <div className="flex flex-col gap-2">
      <p className="text-sm font-medium text-foreground">
        Added {report.panelsAdded} widget
        {report.panelsAdded === 1 ? "" : "s"} and {report.variablesAdded}{" "}
        variable{report.variablesAdded === 1 ? "" : "s"}.
      </p>
      {report.failures.length > 0 ? (
        <ul className="flex flex-col gap-1 rounded-lg border border-destructive/30 bg-destructive/5 p-2 text-xs text-destructive">
          {report.failures.map((f, i) => (
            <li key={i}>{f}</li>
          ))}
        </ul>
      ) : null}
      <Button variant="outline" size="sm" onClick={onOpen}>
        Open dashboard
      </Button>
    </div>
  );
}
