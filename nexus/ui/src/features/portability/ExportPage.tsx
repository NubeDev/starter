import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  ArrowLeft,
  Check,
  ClipboardCopy,
  Download,
} from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";
import { useExportDashboard } from "@/features/dashboards/useDashboardPortability";
import {
  exportToJson,
  filterExport,
  selectionCount,
  selectionTotals,
} from "@/features/portability/model";
import { PlacementPreview } from "@/features/portability/PlacementPreview";
import { VariableSelectList } from "@/features/portability/VariableSelectList";
import { useSelection } from "@/features/portability/useSelection";
import {
  copyToClipboard,
  downloadTextFile,
  exportFilename,
} from "@/features/portability/fileOps";

// A dedicated, full-page export flow: a stripped-back view of the dashboard
// showing its widgets in place, where the user ticks exactly which widgets and
// variables to take, previews the result, then downloads or copies the JSON.
// Selective export is pure client-side filtering of the full export model, so
// no extra backend call is needed (the model is self-contained).
export function ExportPage() {
  const { slug = "" } = useParams();
  const exportDashboard = useExportDashboard();
  const { mutate: fetchExport } = exportDashboard;
  const model = exportDashboard.data;

  // Fetch the full portable model once on mount (a mutation used as a one-shot
  // load — matches how the rest of the app triggers export).
  useEffect(() => {
    if (slug) fetchExport(slug);
  }, [slug, fetchExport]);

  const { selection, togglePanel, toggleVariable, all, none } =
    useSelection(model);

  const filtered = useMemo(
    () => (model ? filterExport(model, selection) : undefined),
    [model, selection],
  );

  if (exportDashboard.isPending && !model) {
    return <Loading label="Preparing export…" />;
  }
  if (exportDashboard.isError) {
    return (
      <ErrorState
        title="Couldn't load this dashboard"
        message={
          exportDashboard.error instanceof Error
            ? exportDashboard.error.message
            : undefined
        }
      />
    );
  }
  if (!model || !filtered) return <Loading label="Preparing export…" />;

  const totals = selectionTotals(model);
  const counts = selectionCount(selection);

  return (
    <div className="flex h-full flex-col gap-4">
      <ExportHeader slug={slug} name={model.name} />

      <div className="grid min-h-0 flex-1 gap-4 lg:grid-cols-[1fr_22rem]">
        {/* Left: the stripped-back board — widgets in their real placement, each
            a toggle for inclusion. */}
        <section className="flex min-h-0 flex-col gap-3 overflow-y-auto">
          <SelectionToolbar
            label="Widgets"
            selected={counts.panels}
            total={totals.panels}
            onAll={all}
            onNone={none}
          />
          <PlacementPreview
            panels={model.panels}
            selection={selection}
            onToggle={togglePanel}
          />
        </section>

        {/* Right: variables + the export actions. */}
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

          <ExportActions slug={slug} json={exportToJson(filtered)} counts={counts} />
        </aside>
      </div>
    </div>
  );
}

function ExportHeader({ slug, name }: { slug: string; name: string }) {
  return (
    <header className="flex items-center justify-between gap-3">
      <div className="flex items-center gap-3">
        <Button asChild variant="ghost" size="sm" className="gap-2">
          <Link to={`/d/${slug}`}>
            <ArrowLeft className="size-4" />
            Back
          </Link>
        </Button>
        <div>
          <h1 className="text-base font-semibold tracking-tight">
            Export “{name}”
          </h1>
          <p className="text-xs text-muted-foreground">
            Choose the widgets and variables to take, then download or copy.
          </p>
        </div>
      </div>
    </header>
  );
}

function SelectionToolbar({
  label,
  selected,
  total,
  onAll,
  onNone,
}: {
  label: string;
  selected: number;
  total: number;
  onAll: () => void;
  onNone: () => void;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <h3 className="text-sm font-semibold">
        {label}{" "}
        <span className="font-normal text-muted-foreground">
          {selected} of {total}
        </span>
      </h3>
      <div className="flex gap-1">
        <Button variant="ghost" size="sm" onClick={onAll}>
          Select all
        </Button>
        <Button variant="ghost" size="sm" onClick={onNone}>
          Clear
        </Button>
      </div>
    </div>
  );
}

function ExportActions({
  slug,
  json,
  counts,
}: {
  slug: string;
  json: string;
  counts: { total: number };
}) {
  const [copied, setCopied] = useState(false);
  const disabled = counts.total === 0;

  return (
    <section className="mt-auto flex flex-col gap-2 rounded-xl border border-border bg-card p-3">
      <p className="text-xs text-muted-foreground">
        {disabled
          ? "Select at least one widget or variable to export."
          : `${counts.total} item${counts.total === 1 ? "" : "s"} ready to export.`}
      </p>
      <div className="flex gap-2">
        <Button
          className="flex-1 gap-2"
          disabled={disabled}
          onClick={() => downloadTextFile(exportFilename(slug), json)}
        >
          <Download className="size-4" />
          Download JSON
        </Button>
        <Button
          variant="outline"
          className="gap-2"
          disabled={disabled}
          onClick={async () => {
            if (await copyToClipboard(json)) {
              setCopied(true);
              setTimeout(() => setCopied(false), 1600);
            }
          }}
        >
          {copied ? (
            <Check className="size-4" />
          ) : (
            <ClipboardCopy className="size-4" />
          )}
          {copied ? "Copied" : "Copy"}
        </Button>
      </div>
    </section>
  );
}
