import { useEffect, useState, type FormEvent } from "react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
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
import { Textarea } from "@nube/starter-ui-kit/components/textarea";

import type {
  CreateDetectionRequest,
  DetectionDetail,
  UpdateDetectionRequest,
} from "@/api/types";
import {
  useDetectionMutations,
  useInsightOptions,
} from "@/features/detections/useDetections";
import { useDatasources } from "@/features/datasources/useDatasources";

// Sentinel for "no datasource" in the picker — Radix Select forbids an empty
// value, so the dev pool / control-plane DB is this explicit option.
const DEV_POOL = "__dev__";

type FormState = {
  name: string;
  insight_id: string;
  datasource_id: string;
  sql: string;
  params: string;
  sources: string;
  flag_column: string;
  target_columns: string;
  value_column: string;
  interval_secs: string;
};

const EMPTY_FORM: FormState = {
  name: "",
  insight_id: "",
  datasource_id: DEV_POOL,
  sql: "",
  params: "{}",
  sources: "",
  flag_column: "",
  target_columns: "",
  value_column: "",
  interval_secs: "300",
};

// Map an existing detection back into the editable form (the inverse of the
// submit packing): jsonb → pretty JSON text, string[] → comma list.
function formFrom(d: DetectionDetail): FormState {
  const sources = Array.isArray(d.sources) ? d.sources : [];
  return {
    name: d.name,
    insight_id: d.insight_id,
    datasource_id: d.datasource_id ?? DEV_POOL,
    sql: d.sql,
    params: JSON.stringify(d.params ?? {}),
    sources: sources.length > 0 ? JSON.stringify(sources) : "",
    flag_column: d.flag_column ?? "",
    target_columns: (d.target_columns ?? []).join(", "),
    value_column: d.value_column ?? "",
    interval_secs: String(d.interval_secs ?? 300),
  };
}

// Create or edit a detection: pick the insight (the Rhai rule), give it a query,
// map which output column is the flag and which identify the target, and set the
// schedule. The insight is authored separately in the Insights Workbench; this
// editor only references it + adds the detection-specific wiring (WS-15 §5).
// Passing `detection` switches the dialog to edit mode (prefilled, PUTs a patch).
export function DetectionDialog({
  open,
  onOpenChange,
  detection,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  detection?: DetectionDetail;
}) {
  const { create, update } = useDetectionMutations();
  const insights = useInsightOptions();
  const datasources = useDatasources();
  const editing = detection != null;
  const [form, setForm] = useState<FormState>(EMPTY_FORM);
  const [paramsError, setParamsError] = useState<string | null>(null);
  const [sourcesError, setSourcesError] = useState<string | null>(null);

  // Re-seed the form whenever the dialog opens (or the target detection changes):
  // prefilled for an edit, blank for a create. Keyed on open so reopening the
  // create dialog after an edit starts clean.
  useEffect(() => {
    if (!open) return;
    setForm(detection ? formFrom(detection) : EMPTY_FORM);
    setParamsError(null);
    setSourcesError(null);
  }, [open, detection]);

  const set = (k: keyof FormState) => (v: string) =>
    setForm((f) => ({ ...f, [k]: v }));

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    let params: unknown = {};
    if (form.params.trim()) {
      try {
        params = JSON.parse(form.params);
      } catch {
        setParamsError("Params must be valid JSON.");
        return;
      }
    }
    setParamsError(null);
    // Optional federated sources: an alias→datasource array the SQL joins over.
    // Blank = single-datasource push-down (the common case); a JSON array opts
    // into the federation engine, exactly like a panel query's `sources`.
    let sources: unknown = [];
    if (form.sources.trim()) {
      try {
        sources = JSON.parse(form.sources);
        if (!Array.isArray(sources)) throw new Error("not an array");
      } catch {
        setSourcesError('Sources must be a JSON array, e.g. [{"alias":"a","datasource":"<id>","table":"t"}].');
        return;
      }
    }
    setSourcesError(null);
    const targets = form.target_columns
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    const value = form.value_column.trim();
    const devPool = form.datasource_id === DEV_POOL;
    const common = {
      name: form.name.trim(),
      insight_id: form.insight_id,
      sql: form.sql.trim(),
      params,
      sources,
      // Empty flag column = "every returned row is a finding" (the filter_gt
      // pattern); a named column gates on its truthiness (the anomalies pattern).
      flag_column: form.flag_column.trim(),
      target_columns: targets,
      value_column: value === "" ? null : value,
      interval_secs: Number(form.interval_secs) || 300,
    };
    const done = { onSuccess: () => onOpenChange(false) };
    if (editing) {
      // The update path uses a clear_datasource flag, not a null id — an explicit
      // JSON null can't be told from "absent" on the wire (same reason the panel
      // editor uses clear_insight). Dev pool ⇒ clear; otherwise set the id.
      const patch: UpdateDetectionRequest = {
        ...common,
        datasource_id: devPool ? undefined : form.datasource_id,
        clear_datasource: devPool,
      };
      update.mutate({ id: detection.id, patch }, done);
    } else {
      // Create takes a plain optional id: dev pool ⇒ null.
      const body: CreateDetectionRequest = {
        ...common,
        datasource_id: devPool ? null : form.datasource_id,
        enabled: true,
      };
      create.mutate(body, done);
    }
  }

  const mutation = editing ? update : create;

  const canSubmit =
    form.name.trim() !== "" &&
    form.insight_id !== "" &&
    form.sql.trim() !== "";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass max-w-lg">
        <DialogHeader>
          <DialogTitle>{editing ? "Edit detection" : "New detection"}</DialogTitle>
          <DialogDescription>
            Run an insight on a schedule and record a finding per flagged row.
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-3" onSubmit={onSubmit}>
          <div className="space-y-1.5">
            <Label htmlFor="det-name">Name</Label>
            <Input
              id="det-name"
              value={form.name}
              onChange={(e) => set("name")(e.target.value)}
              required
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="det-insight">Insight (the rule)</Label>
            <Select value={form.insight_id} onValueChange={set("insight_id")}>
              <SelectTrigger id="det-insight">
                <SelectValue placeholder="Pick an insight…" />
              </SelectTrigger>
              <SelectContent>
                {(insights.data ?? []).map((i) => (
                  <SelectItem key={i.id} value={i.id}>
                    {i.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {insights.data && insights.data.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                No insights yet — author one in the Insights Workbench first.
              </p>
            ) : null}
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="det-datasource">Datasource</Label>
            <Select
              value={form.datasource_id}
              onValueChange={set("datasource_id")}
            >
              <SelectTrigger id="det-datasource">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={DEV_POOL}>Dev pool (control-plane DB)</SelectItem>
                {(datasources.data ?? []).map((d) => (
                  <SelectItem key={d.id} value={d.id}>
                    {d.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              The database the query runs against. Use federated sources below to
              join across datasources.
            </p>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="det-sql">Query</Label>
            <Textarea
              id="det-sql"
              value={form.sql}
              onChange={(e) => set("sql")(e.target.value)}
              placeholder="select meter, value from telemetry_typed where ts > now() - interval '1h'"
              spellCheck={false}
              className="min-h-20 resize-y font-mono text-sm"
              required
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label htmlFor="det-flag">Flag column</Label>
              <Input
                id="det-flag"
                value={form.flag_column}
                onChange={(e) => set("flag_column")(e.target.value)}
                placeholder="value_anomaly (blank = every row)"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="det-value">Value column</Label>
              <Input
                id="det-value"
                value={form.value_column}
                onChange={(e) => set("value_column")(e.target.value)}
                placeholder="value"
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="det-targets">Target columns (comma-separated)</Label>
            <Input
              id="det-targets"
              value={form.target_columns}
              onChange={(e) => set("target_columns")(e.target.value)}
              placeholder="site, meter"
            />
            <p className="text-xs text-muted-foreground">
              These identify each finding and dedupe re-flags into one open
              finding per target.
            </p>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label htmlFor="det-interval">Run every (seconds)</Label>
              <Input
                id="det-interval"
                type="number"
                value={form.interval_secs}
                onChange={(e) => set("interval_secs")(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="det-params">Params (JSON)</Label>
              <Input
                id="det-params"
                value={form.params}
                onChange={(e) => set("params")(e.target.value)}
                placeholder='{ "limit": 100 }'
                className="font-mono text-sm"
              />
            </div>
          </div>
          <p className="text-xs text-muted-foreground">
            Tip: a float param like a z-score must be written{" "}
            <code>2.0</code> (or coerced in-script with <code>* 1.0</code>) so it
            doesn't round-trip as an integer.
          </p>

          <div className="space-y-1.5">
            <Label htmlFor="det-sources">Federated sources (optional, JSON)</Label>
            <Textarea
              id="det-sources"
              value={form.sources}
              onChange={(e) => set("sources")(e.target.value)}
              placeholder='[{ "alias": "pg", "datasource": "<uuid>", "table": "usage" }]'
              spellCheck={false}
              className="min-h-16 resize-y font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground">
              Leave blank to query a single datasource. Provide an alias→datasource
              array to join across datasources or files — each alias is the SQL
              table <code>ds_&lt;alias&gt;</code>, same as a federated panel query.
            </p>
          </div>

          {paramsError ? (
            <p role="alert" className="text-sm text-destructive">
              {paramsError}
            </p>
          ) : null}
          {sourcesError ? (
            <p role="alert" className="text-sm text-destructive">
              {sourcesError}
            </p>
          ) : null}
          {mutation.isError ? (
            <p role="alert" className="text-sm text-destructive">
              {mutation.error instanceof Error
                ? mutation.error.message
                : `Couldn't ${editing ? "save" : "create"} the detection.`}
            </p>
          ) : null}

          <DialogFooter>
            <Button type="submit" disabled={mutation.isPending || !canSubmit}>
              {mutation.isPending
                ? editing
                  ? "Saving…"
                  : "Creating…"
                : editing
                  ? "Save detection"
                  : "Create detection"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
