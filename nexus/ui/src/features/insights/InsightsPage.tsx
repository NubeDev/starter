import { useMemo, useState } from "react";
import { Save } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import type { QueryResponse } from "@/api/types";
import { useInsightFunctions } from "@/features/insights/useInsightFunctions";
import { usePreviewInsight } from "@/features/insights/usePreviewInsight";
import { SourcePane } from "@/features/insights/SourcePane";
import { TransformPane } from "@/features/insights/TransformPane";
import { ResultPane } from "@/features/insights/ResultPane";
import { SaveInsightDialog } from "@/features/insights/SaveInsightDialog";

// Insights Workbench — a 3-pane data notebook for authoring Rhai transforms
// with instant feedback. Load real rows (Source) → write a transform
// (Transform, with catalogue autocomplete + cheatsheet) → SEE the result
// update live (Result, debounced `/insights/preview`) → save it.
//
// This component owns the shared state the three panes coordinate on: the input
// sample, the script, and the params JSON (parsed once here so a parse error
// gates preview and shows inline). The route at `/insights` still mounts this
// component, so the page stays mountable.
export function InsightsPage() {
  const functions = useInsightFunctions();
  const [sample, setSample] = useState<QueryResponse | null>(null);
  const [script, setScript] = useState("");
  const [paramsText, setParamsText] = useState("");
  const [saveOpen, setSaveOpen] = useState(false);

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

  return (
    <div className="flex h-full min-h-0 flex-col gap-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-semibold tracking-tight">
            Insights Workbench
          </h2>
          <p className="text-xs text-muted-foreground">
            Load data, write a transform, and see the result update live.
          </p>
        </div>
        <Button
          size="sm"
          className="gap-2"
          disabled={!script.trim()}
          onClick={() => setSaveOpen(true)}
        >
          <Save className="size-4" />
          Save as insight
        </Button>
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
      />
    </div>
  );
}
