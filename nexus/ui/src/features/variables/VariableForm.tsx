import { useState } from "react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Checkbox } from "@nube/starter-ui-kit/components/checkbox";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import type { CreateVariableRequest, VariableDetail } from "@/api/types";
import type { ContextSource, VariableKind } from "@/data/types";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { SqlEditor } from "@/features/sql-editor";
import { parseKindConfig, toOptionsConfig } from "@/features/variables/config";

const KINDS: { value: VariableKind; label: string; help: string }[] = [
  { value: "custom", label: "Custom", help: "A static, comma-separated option list." },
  { value: "query", label: "Query", help: "Options from SQL run against a datasource." },
  { value: "datasource", label: "Datasource", help: "The tenant's datasources of a kind." },
  { value: "interval", label: "Interval", help: "A list of durations for $__interval." },
  { value: "textbox", label: "Text box", help: "A free-text value." },
  { value: "constant", label: "Constant", help: "A fixed (usually hidden) value." },
  {
    value: "context",
    label: "Context",
    help: "A value read from the page's nav node, URL, tags, or mount values.",
  },
];

const CONTEXT_SOURCES: { value: ContextSource; label: string; help: string }[] = [
  { value: "values", label: "Mount values", help: "The nav node's context.values (e.g. building)." },
  { value: "url", label: "URL param", help: "A bare ?key=… query param (deep links)." },
  { value: "tag", label: "Dashboard tag", help: "This dashboard's tag value for the key." },
  { value: "nav", label: "Nav node", help: "slug, name, or path[n] of the nav node." },
];

// The authoring form for one variable (item 4): name/label, kind, the
// kind-specific config, and the multi/All/hidden flags. It produces a
// `CreateVariableRequest`-shaped payload on submit; the dialog decides
// whether that's a create or an update. Kind-specific config is held as a
// typed value and serialised to the opaque `options_config` on submit.
export function VariableForm({
  initial,
  onSubmit,
  onCancel,
  submitLabel,
}: {
  initial?: VariableDetail;
  onSubmit: (payload: CreateVariableRequest) => void;
  onCancel: () => void;
  submitLabel: string;
}) {
  const [name, setName] = useState(initial?.name ?? "");
  const [label, setLabel] = useState(initial?.label ?? "");
  const [kind, setKind] = useState<VariableKind>(initial?.kind ?? "custom");
  const [multi, setMulti] = useState(initial?.multi ?? false);
  const [includeAll, setIncludeAll] = useState(initial?.include_all ?? false);
  const [hidden, setHidden] = useState(initial?.hidden ?? false);

  // Kind config fields, seeded from the existing variable when editing.
  const seeded = parseKindConfig(kind, initial?.options_config);
  const [customText, setCustomText] = useState(
    seeded.kind === "custom" ? seeded.optionsText : "",
  );
  const [constantValue, setConstantValue] = useState(
    seeded.kind === "constant" ? seeded.value : "",
  );
  const [intervalSteps, setIntervalSteps] = useState(
    seeded.kind === "interval" ? seeded.steps.join(", ") : "1m, 5m, 1h",
  );
  const [textboxDefault, setTextboxDefault] = useState(
    seeded.kind === "textbox" ? seeded.default : "",
  );
  const [dsFilter, setDsFilter] = useState(
    seeded.kind === "datasource" ? (seeded.kindFilter ?? "") : "",
  );
  const [querySql, setQuerySql] = useState(seeded.kind === "query" ? seeded.sql : "");
  const [queryDs, setQueryDs] = useState(
    seeded.kind === "query" ? seeded.datasourceId : "",
  );
  const [contextSource, setContextSource] = useState<ContextSource>(
    seeded.kind === "context" ? seeded.source : "values",
  );
  const [contextKey, setContextKey] = useState(
    seeded.kind === "context" ? seeded.key : "",
  );

  const nameValid = /^[a-zA-Z][a-zA-Z0-9_]*$/.test(name);

  function buildConfig(): Record<string, unknown> {
    switch (kind) {
      case "custom":
        return toOptionsConfig({ kind, optionsText: customText });
      case "constant":
        return toOptionsConfig({ kind, value: constantValue });
      case "interval":
        return toOptionsConfig({
          kind,
          steps: intervalSteps.split(",").map((s) => s.trim()).filter(Boolean),
        });
      case "textbox":
        return toOptionsConfig({ kind, default: textboxDefault });
      case "datasource":
        return toOptionsConfig({ kind, kindFilter: dsFilter || undefined });
      case "query":
        return toOptionsConfig({ kind, sql: querySql, datasourceId: queryDs });
      case "context":
        return toOptionsConfig({
          kind,
          source: contextSource,
          key: contextKey.trim(),
        });
    }
  }

  function submit() {
    if (!nameValid) return;
    onSubmit({
      name,
      label: label.trim() || null,
      kind,
      options_config: buildConfig(),
      multi,
      include_all: includeAll,
      hidden,
    });
  }

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1.5">
          <Label htmlFor="vf-name">Name</Label>
          <Input
            id="vf-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="region"
            aria-invalid={!nameValid && name.length > 0}
          />
          {!nameValid && name.length > 0 ? (
            <p className="text-xs text-destructive">
              Letters, digits and underscore; must start with a letter.
            </p>
          ) : null}
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="vf-label">Label (optional)</Label>
          <Input
            id="vf-label"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="Region"
          />
        </div>
      </div>

      <div className="space-y-1.5">
        <Label htmlFor="vf-kind">Type</Label>
        <Select value={kind} onValueChange={(v) => setKind(v as VariableKind)}>
          <SelectTrigger id="vf-kind">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {KINDS.map((k) => (
              <SelectItem key={k.value} value={k.value}>
                {k.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {KINDS.find((k) => k.value === kind)?.help}
        </p>
      </div>

      {kind === "custom" ? (
        <div className="space-y-1.5">
          <Label htmlFor="vf-custom">Options</Label>
          <Input
            id="vf-custom"
            value={customText}
            onChange={(e) => setCustomText(e.target.value)}
            placeholder="prod, staging, Dev : dev"
          />
          <p className="text-xs text-muted-foreground">
            Comma-separated. Use <code>text : value</code> to label an option.
          </p>
        </div>
      ) : null}

      {kind === "constant" ? (
        <div className="space-y-1.5">
          <Label htmlFor="vf-const">Value</Label>
          <Input
            id="vf-const"
            value={constantValue}
            onChange={(e) => setConstantValue(e.target.value)}
          />
        </div>
      ) : null}

      {kind === "interval" ? (
        <div className="space-y-1.5">
          <Label htmlFor="vf-interval">Steps</Label>
          <Input
            id="vf-interval"
            value={intervalSteps}
            onChange={(e) => setIntervalSteps(e.target.value)}
            placeholder="1m, 5m, 1h"
          />
        </div>
      ) : null}

      {kind === "textbox" ? (
        <div className="space-y-1.5">
          <Label htmlFor="vf-textbox">Default</Label>
          <Input
            id="vf-textbox"
            value={textboxDefault}
            onChange={(e) => setTextboxDefault(e.target.value)}
          />
        </div>
      ) : null}

      {kind === "datasource" ? (
        <div className="space-y-1.5">
          <Label htmlFor="vf-dsfilter">Datasource kind filter (optional)</Label>
          <Input
            id="vf-dsfilter"
            value={dsFilter}
            onChange={(e) => setDsFilter(e.target.value)}
            placeholder="postgres"
          />
        </div>
      ) : null}

      {kind === "query" ? (
        <div className="space-y-2">
          <div className="space-y-1.5">
            <Label>Datasource</Label>
            <DatasourcePicker value={queryDs || undefined} onChange={setQueryDs} />
          </div>
          <div className="space-y-1.5">
            <Label>Option query</Label>
            <SqlEditor value={querySql} onChange={setQuerySql} />
            <p className="text-xs text-muted-foreground">
              Returns one column of option values. May reference another
              variable (<code>$parent</code>) to cascade.
            </p>
          </div>
        </div>
      ) : null}

      {kind === "context" ? (
        <div className="space-y-2">
          <div className="space-y-1.5">
            <Label htmlFor="vf-ctx-source">Source</Label>
            <Select
              value={contextSource}
              onValueChange={(v) => setContextSource(v as ContextSource)}
            >
              <SelectTrigger id="vf-ctx-source">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {CONTEXT_SOURCES.map((s) => (
                  <SelectItem key={s.value} value={s.value}>
                    {s.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              {CONTEXT_SOURCES.find((s) => s.value === contextSource)?.help}
            </p>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="vf-ctx-key">Key</Label>
            <Input
              id="vf-ctx-key"
              value={contextKey}
              onChange={(e) => setContextKey(e.target.value)}
              placeholder={
                contextSource === "nav" ? "slug · name · path[0]" : "building"
              }
            />
            <p className="text-xs text-muted-foreground">
              {contextSource === "nav"
                ? "slug, name, or path[n] of the nav node."
                : `The key to read from the ${
                    contextSource === "values"
                      ? "nav node's mount values"
                      : contextSource === "url"
                        ? "URL query string"
                        : "dashboard's tags"
                  }.`}
            </p>
          </div>
        </div>
      ) : null}

      <div className="flex flex-wrap gap-4">
        {kind !== "textbox" && kind !== "constant" && kind !== "context" ? (
          <label className="flex items-center gap-2 text-sm">
            <Checkbox checked={multi} onCheckedChange={(c) => setMulti(c === true)} />
            Allow multiple
          </label>
        ) : null}
        {multi ? (
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={includeAll}
              onCheckedChange={(c) => setIncludeAll(c === true)}
            />
            Include “All”
          </label>
        ) : null}
        <label className="flex items-center gap-2 text-sm">
          <Checkbox checked={hidden} onCheckedChange={(c) => setHidden(c === true)} />
          Hide from bar
        </label>
      </div>

      <div className="flex justify-end gap-2">
        <Button variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button onClick={submit} disabled={!nameValid}>
          {submitLabel}
        </Button>
      </div>
    </div>
  );
}
