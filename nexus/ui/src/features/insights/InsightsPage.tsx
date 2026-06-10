import { useEffect, useMemo, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { ArrowLeft, Check, Save } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import type { QueryResponse } from "@/api/types";
import { useInsight } from "@/features/insights/useInsight";
import { useUpdateInsight } from "@/features/insights/useInsightMutations";
import { useInsightFunctions } from "@/features/insights/useInsightFunctions";
import { usePreviewInsight } from "@/features/insights/usePreviewInsight";
import { SourcePane } from "@/features/insights/SourcePane";
import { TransformPane } from "@/features/insights/TransformPane";
import { ResultPane } from "@/features/insights/ResultPane";
import { SaveInsightDialog } from "@/features/insights/SaveInsightDialog";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Insights Workbench — a 3-pane data notebook for authoring Rhai transforms
// with instant feedback. Load real rows (Source) → write a transform
// (Transform, with catalogue autocomplete + cheatsheet) → SEE the result
// update live (Result, debounced `/insights/preview`) → save it.
//
// Two modes, selected by the `?id=` query param:
//  - NEW (no id): "Save as insight" creates a record, then returns to the list.
//  - EDIT (`?id=…`): the saved script is pre-filled; "Save changes" PATCHes the
//    existing record in place. The list owns rename + delete; the Workbench owns
//    the script, where it can be compile-checked against a live sample.
//
// This component owns the shared state the three panes coordinate on: the input
// sample, the script, and the params JSON (parsed once here so a parse error
// gates preview and shows inline).
export function InsightsPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const editingId = searchParams.get("id") ?? undefined;
  const editing = useInsight(editingId);

  const functions = useInsightFunctions();
  const [sample, setSample] = useState<QueryResponse | null>(null);
  const [script, setScript] = useState("");
  const [paramsText, setParamsText] = useState("");
  const [saveOpen, setSaveOpen] = useState(false);

  // In edit mode, seed the editor from the loaded insight exactly once (when it
  // arrives). After that the user owns the script state, so we key the seed on
  // the insight id and only run it on first load for that id.
  const [seededFor, setSeededFor] = useState<string | null>(null);
  useEffect(() => {
    if (editing.data && seededFor !== editing.data.id) {
      setScript(editing.data.script);
      setSeededFor(editing.data.id);
    }
  }, [editing.data, seededFor]);

  // Parse the params JSON once: success feeds preview, failure gates it and is
  // shown inline in the Transform pane. An empty box means "no params".
  const { params, paramsError } = useMemo(() => {
    if (!paramsText.trim()) return { params: undefined, paramsError: null };
    try {
      return { params: JSON.parse(paramsText) as unknown, paramsError: null };
    } catch (e) {
      return {
        params: undefined,
        paramsError: `Invalid JSON: ${e instanceof Error ? e.message : String(e)}`,
      };
    }
  }, [paramsText]);

  const preview = usePreviewInsight({
    sample,
    script,
    params,
    // Skip preview while the params box has a parse error — we'd be sending
    // stale/no params and the inline error already tells the user what to fix.
    enabled: paramsError === null,
  });

  const update = useUpdateInsight();

  // Loading / error gates for edit mode — don't show a half-seeded editor.
  if (editingId && editing.isPending) {
    return <Loading label="Loading insight…" />;
  }
  if (editingId && editing.isError) {
    return (
      <ErrorState
        message={
          editing.error instanceof Error ? editing.error.message : undefined
        }
      />
    );
  }

  const isEdit = !!editingId && !!editing.data;

  return (
    <div className="flex h-full min-h-0 flex-col gap-4">
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <Button
            variant="ghost"
            size="sm"
            className="gap-1.5"
            onClick={() => navigate("/insights")}
          >
            <ArrowLeft className="size-4" />
            Insights
          </Button>
          <div>
            <h2 className="text-base font-semibold tracking-tight">
              {isEdit ? `Edit · ${editing.data?.name}` : "Insights Workbench"}
            </h2>
            <p className="text-xs text-muted-foreground">
              Load data, write a transform, and see the result update live.
            </p>
          </div>
        </div>

        {isEdit ? (
          <div className="flex items-center gap-2">
            {update.isError ? (
              <span role="alert" className="text-xs text-destructive">
                {update.error instanceof Error
                  ? update.error.message
                  : "Couldn't save changes."}
              </span>
            ) : null}
            <Button
              size="sm"
              className="gap-2"
              disabled={!script.trim() || update.isPending}
              onClick={() => {
                if (!editingId) return;
                update.mutate(
                  { id: editingId, body: { script } },
                  { onSuccess: () => navigate("/insights") },
                );
              }}
            >
              {update.isPending ? (
                "Saving…"
              ) : (
                <>
                  <Check className="size-4" />
                  Save changes
                </>
              )}
            </Button>
          </div>
        ) : (
          <Button
            size="sm"
            className="gap-2"
            disabled={!script.trim()}
            onClick={() => setSaveOpen(true)}
          >
            <Save className="size-4" />
            Save as insight
          </Button>
        )}
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-4 lg:grid-cols-3">
        <SourcePane
          sample={sample}
          onSample={setSample}
          onLoadScript={setScript}
          onLoadTemplate={setScript}
        />
        <TransformPane
          script={script}
          onScriptChange={setScript}
          functions={functions.data ?? []}
          functionsLoading={functions.isPending}
          paramsText={paramsText}
          onParamsChange={setParamsText}
          paramsError={paramsError}
        />
        <ResultPane preview={preview} sample={sample} hasSample={!!sample} />
      </div>

      <SaveInsightDialog
        open={saveOpen}
        onOpenChange={setSaveOpen}
        script={script}
        onSaved={() => navigate("/insights")}
      />
    </div>
  );
}
