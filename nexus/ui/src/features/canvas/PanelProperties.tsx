import { useState, type FormEvent } from "react";
import { useMutation } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import { PlayCircle, X } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
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

import type { Widget, WidgetType } from "@/data/types";
import type { QueryResponse } from "@/api/types";
import { queryDatasource } from "@/api/datasources/query";
import { runQuery } from "@/api/query/run";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { WIDGET_CATALOG, WIDGET_TYPES } from "@/features/widgets/catalog";
import { useUpdatePanel } from "@/features/dashboards/useUpdatePanel";

const needsX = (t: WidgetType) => WIDGET_CATALOG[t].roles.x === "required";

// The properties panel: view + edit a selected panel's configuration
// (type, title, datasource, SQL, field mapping). Lives in the same
// edit-mode side slot as the palette and is shown when a panel is
// selected. Seeded from the widget's current config; saving PATCHes the
// panel and the canvas re-renders from the refreshed dashboard.
//
// Mounted with a `key={widget.id}` by the parent so selecting a different
// panel remounts this with fresh initial state — no stale-field effects.
export function PanelProperties({
  widget,
  slug,
  onClose,
}: {
  widget: Widget;
  slug: string;
  onClose: () => void;
}) {
  const update = useUpdatePanel(slug);
  const [type, setType] = useState<WidgetType>(widget.type);
  const [title, setTitle] = useState(widget.title);
  const [datasourceId, setDatasourceId] = useState<string | undefined>(
    widget.config.query.datasourceId || undefined,
  );
  const [sql, setSql] = useState(widget.config.query.sql);
  const [xCol, setXCol] = useState(widget.config.fields.x ?? "");
  const [valueCol, setValueCol] = useState(
    widget.config.fields.series[0]?.value ?? "",
  );

  // Test query: run the *current form* SQL against the selected datasource
  // on demand (a mutation, so it fires only on click — not on keystroke).
  // Uses the same query paths panels render from; the result is a quick
  // confidence check (row count / columns / error) before saving, and
  // never mutates the panel.
  const client = useStarterClient();
  const test = useMutation<QueryResponse, Error>({
    mutationFn: () => {
      const req = { sql: sql.trim() };
      return datasourceId
        ? queryDatasource(client, datasourceId, req)
        : runQuery(client, req);
    },
  });
  const canTest = Boolean(datasourceId) && sql.trim().length > 0;

  const ready = title.trim() && datasourceId && sql.trim() && valueCol.trim();

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!ready) return;
    // Preserve any series metadata beyond the first value column (labels,
    // units, colours the user set elsewhere); only the primary value and
    // the x column are editable here.
    const [first, ...rest] = widget.config.fields.series;
    const next: Widget = {
      ...widget,
      type,
      title: title.trim(),
      config: {
        ...widget.config,
        query: { ...widget.config.query, datasourceId: datasourceId!, sql: sql.trim() },
        fields: {
          ...widget.config.fields,
          x: needsX(type) ? xCol.trim() || undefined : undefined,
          series: [{ ...(first ?? {}), value: valueCol.trim() }, ...rest],
        },
      },
    };
    update.mutate(next, { onSuccess: onClose });
  }

  return (
    <aside className="glass flex w-72 shrink-0 flex-col gap-3 overflow-y-auto rounded-xl p-3">
      <header className="flex items-center justify-between">
        <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          Panel properties
        </h3>
        <button
          type="button"
          aria-label="Close properties"
          onClick={onClose}
          className="rounded p-0.5 text-muted-foreground transition-colors hover:bg-accent/40 hover:text-foreground"
        >
          <X className="size-4" />
        </button>
      </header>

      <form className="space-y-3" onSubmit={onSubmit}>
        <div className="space-y-1.5">
          <Label htmlFor="prop-type">Type</Label>
          <Select value={type} onValueChange={(v) => setType(v as WidgetType)}>
            <SelectTrigger id="prop-type">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {WIDGET_TYPES.map((t) => (
                <SelectItem key={t} value={t}>
                  {WIDGET_CATALOG[t].label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="prop-title">Title</Label>
          <Input
            id="prop-title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            required
          />
        </div>

        <div className="space-y-1.5">
          <Label>Datasource</Label>
          <DatasourcePicker value={datasourceId} onChange={setDatasourceId} />
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="prop-sql">SQL</Label>
          <Textarea
            id="prop-sql"
            value={sql}
            onChange={(e) => setSql(e.target.value)}
            spellCheck={false}
            className="min-h-24 resize-y font-mono text-sm"
          />
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="w-full gap-2"
            disabled={!canTest || test.isPending}
            onClick={() => test.mutate()}
          >
            <PlayCircle className="size-4" />
            {test.isPending ? "Running…" : "Test query"}
          </Button>
          {test.isError ? (
            <p
              role="alert"
              className="rounded-md bg-destructive/10 px-2 py-1.5 text-xs text-destructive"
            >
              {test.error instanceof Error ? test.error.message : "Query failed."}
            </p>
          ) : test.isSuccess ? (
            <div className="rounded-md bg-accent/20 px-2 py-1.5 text-xs text-muted-foreground">
              <p className="text-foreground">
                {test.data.stats.row_count} row
                {test.data.stats.row_count === 1 ? "" : "s"} ·{" "}
                {test.data.stats.elapsed_ms} ms
                {test.data.stats.truncated ? " · capped" : ""}
              </p>
              {test.data.columns.length > 0 ? (
                <p className="mt-0.5 truncate font-mono">
                  {test.data.columns.map((c) => c.name).join(", ")}
                </p>
              ) : null}
            </div>
          ) : null}
        </div>

        {needsX(type) ? (
          <div className="space-y-1.5">
            <Label htmlFor="prop-x">X column</Label>
            <Input
              id="prop-x"
              value={xCol}
              onChange={(e) => setXCol(e.target.value)}
              placeholder="ts"
            />
          </div>
        ) : null}

        <div className="space-y-1.5">
          <Label htmlFor="prop-value">Value column</Label>
          <Input
            id="prop-value"
            value={valueCol}
            onChange={(e) => setValueCol(e.target.value)}
            placeholder="value"
            required
          />
        </div>

        {update.isError ? (
          <p role="alert" className="text-sm text-destructive">
            Couldn't save the panel.
          </p>
        ) : null}

        <Button
          type="submit"
          className="w-full"
          disabled={!ready || update.isPending}
        >
          {update.isPending ? "Saving…" : "Save changes"}
        </Button>
      </form>
    </aside>
  );
}
