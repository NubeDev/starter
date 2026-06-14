import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import { useInsights } from "@/features/insights/useInsights";

// What kind of insight the explorer attaches to a run: nothing (default),
// a stored tenant insight by id, or an inline Rhai script.
export type InsightMode = "none" | "stored" | "script";

// Sentinel SelectItem value for the "no insight" option — Select can't take
// an empty-string value, so we map this to InsightMode "none".
const NONE_VALUE = "__none__";

// Optional post-query insight selector. The chosen insight transforms the
// result rows after the query runs (never adds rows). Stored insights come
// from `GET /insights`; the inline case lets the user paste a Rhai script.
// Params (shared by both) are authored as JSON one level up so a parse error
// can gate the run.
export function InsightPicker({
  mode,
  onModeChange,
  insightId,
  onInsightIdChange,
  script,
  onScriptChange,
  params,
  onParamsChange,
  paramsError,
}: {
  mode: InsightMode;
  onModeChange: (mode: InsightMode) => void;
  insightId: string | undefined;
  onInsightIdChange: (id: string) => void;
  script: string;
  onScriptChange: (script: string) => void;
  params: string;
  onParamsChange: (params: string) => void;
  paramsError: string | null;
}) {
  const { data, isPending, isError } = useInsights();
  const insights = data ?? [];

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <span className="text-sm text-muted-foreground">Insight</span>
        <Select
          value={mode === "stored" && insightId ? insightId : mode}
          onValueChange={(v) => {
            if (v === NONE_VALUE) {
              onModeChange("none");
            } else if (v === "script") {
              onModeChange("script");
            } else {
              // A stored-insight id was picked.
              onModeChange("stored");
              onInsightIdChange(v);
            }
          }}
        >
          <SelectTrigger className="w-56">
            <SelectValue placeholder="None" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={NONE_VALUE}>None</SelectItem>
            <SelectItem value="script">Inline script…</SelectItem>
            {insights.map((ins) => (
              <SelectItem key={ins.id} value={ins.id}>
                {ins.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        {isPending ? (
          <span className="text-xs text-muted-foreground">Loading…</span>
        ) : isError ? (
          <span className="text-xs text-destructive">
            Failed to load insights
          </span>
        ) : null}
      </div>

      {mode === "script" ? (
        <textarea
          value={script}
          onChange={(e) => onScriptChange(e.target.value)}
          placeholder="Inline Rhai transform script…"
          aria-label="Inline insight script"
          rows={3}
          className="w-full rounded-md border bg-transparent p-2 font-mono text-sm"
        />
      ) : null}

      {mode !== "none" ? (
        <div className="flex flex-col gap-1">
          <textarea
            value={params}
            onChange={(e) => onParamsChange(e.target.value)}
            placeholder='params (optional JSON, e.g. {"window": 7})'
            aria-label="Insight params (JSON)"
            rows={2}
            className="w-full rounded-md border bg-transparent p-2 font-mono text-sm"
          />
          {paramsError ? (
            <span className="text-xs text-destructive">{paramsError}</span>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
