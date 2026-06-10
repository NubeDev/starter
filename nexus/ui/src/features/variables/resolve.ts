// Compute a variable's option list from its kind config (item 6 resolution).
// Static kinds (constant/custom/interval/textbox) resolve synchronously from
// their config; `datasource` resolves from the tenant's datasource list;
// `query` runs option SQL against a datasource, with any referenced parent
// variables passed through as `QueryVariable`s so the WS-03 binder expands
// them safely (cascading) — we never inline parent values into SQL text.

import type { StarterClient } from "@nube/starter-client-ts";

import { listDatasources } from "@/api/datasources/list";
import { queryDatasource } from "@/api/datasources/query";
import type { QueryRequest, QueryVariable } from "@/api/types";
import type { PageContext, VariableKind, VariableOption } from "@/data/types";
import { parseKindConfig } from "@/features/variables/config";
import {
  EMPTY_PAGE_CONTEXT,
  resolveContextValue,
} from "@/features/variables/context";
import { referencedVariables } from "@/features/variables/deps";

/** Current selections of *already-resolved* variables, keyed by name — the
 *  inputs a cascading `query` variable interpolates against. */
export type ResolvedSelections = Record<string, ReadonlyArray<string>>;

/** Parse `custom` authoring text into options. Entries are comma-separated;
 *  each is `value` or `text : value` (Grafana syntax). Blank entries drop. */
export function parseCustomOptions(text: string): VariableOption[] {
  return text
    .split(",")
    .map((raw) => raw.trim())
    .filter((raw) => raw.length > 0)
    .map((entry) => {
      const colon = entry.indexOf(":");
      if (colon === -1) return { text: entry, value: entry };
      const text = entry.slice(0, colon).trim();
      const value = entry.slice(colon + 1).trim();
      return { text: text || value, value };
    });
}

/** Map an interval step list to options (text === value: `1m`, `5m`, …). */
function intervalOptions(steps: ReadonlyArray<string>): VariableOption[] {
  return steps
    .map((s) => s.trim())
    .filter((s) => s.length > 0)
    .map((s) => ({ text: s, value: s }));
}

/** Resolve a variable's option list. Pure/synchronous for static kinds;
 *  for `query`/`datasource` it awaits the relevant fetch. `selections`
 *  supplies parent values for a cascading query variable. */
export async function resolveOptions(
  client: StarterClient,
  kind: VariableKind,
  optionsConfig: unknown,
  selections: ResolvedSelections,
  pageContext: PageContext = EMPTY_PAGE_CONTEXT,
): Promise<VariableOption[]> {
  const cfg = parseKindConfig(kind, optionsConfig);
  switch (cfg.kind) {
    case "constant":
      return cfg.value ? [{ text: cfg.value, value: cfg.value }] : [];
    case "custom":
      return parseCustomOptions(cfg.optionsText);
    case "interval":
      return intervalOptions(cfg.steps);
    case "textbox":
      // A textbox has no fixed list; its default seeds the current value.
      return cfg.default ? [{ text: cfg.default, value: cfg.default }] : [];
    case "context": {
      // The single option a `context` variable resolves to is its value read
      // from the page context — synchronous, no fetch. An absent source/key
      // yields no option (the variable resolves empty, not stale).
      const value = resolveContextValue(cfg, pageContext);
      return value ? [{ text: value, value }] : [];
    }
    case "datasource": {
      const all = await listDatasources(client);
      const filtered = cfg.kindFilter
        ? all.filter((d) => d.kind === cfg.kindFilter)
        : all;
      // The *value* a datasource variable binds is the id (panels target by
      // id); the label is the human name.
      return filtered.map((d) => ({ text: d.name, value: d.id }));
    }
    case "query":
      return resolveQueryOptions(client, cfg.sql, cfg.datasourceId, cfg, selections);
  }
}

/** Run a `query` variable's option SQL and project its rows to options.
 *  Referenced parent variables are sent as `QueryVariable`s so the server
 *  binder interpolates them (`$parent`/`$__sqlIn(parent)`) — never inlined
 *  here — keeping option SQL injection-safe by construction. */
async function resolveQueryOptions(
  client: StarterClient,
  sql: string,
  datasourceId: string,
  cfg: { textColumn?: string; valueColumn?: string },
  selections: ResolvedSelections,
): Promise<VariableOption[]> {
  if (!sql.trim() || !datasourceId) return [];

  // Only pass the parents this SQL actually references and that have a
  // selection — an unselected parent contributes nothing.
  const refs = referencedVariables(sql);
  const variables: QueryVariable[] = refs
    .map((name) => ({ name, values: [...(selections[name] ?? [])] }))
    .filter((v) => v.values.length > 0);

  const request: QueryRequest = { sql, variables };
  const response = await queryDatasource(client, datasourceId, request);

  const cols = response.columns.map((c) => c.name);
  if (cols.length === 0) return [];
  const valueCol =
    (cfg.valueColumn && cols.includes(cfg.valueColumn) && cfg.valueColumn) ||
    cols[0];
  const textCol =
    (cfg.textColumn && cols.includes(cfg.textColumn) && cfg.textColumn) ||
    valueCol;

  const seen = new Set<string>();
  const options: VariableOption[] = [];
  for (const row of response.rows as Array<Record<string, unknown>>) {
    const value = stringify(row[valueCol]);
    if (value === undefined || seen.has(value)) continue;
    seen.add(value);
    const text = stringify(row[textCol]) ?? value;
    options.push({ text, value });
  }
  return options;
}

/** Coerce a JSON cell to the string an option binds; `null`/`undefined`
 *  drop (an option must have a value). */
function stringify(cell: unknown): string | undefined {
  if (cell === null || cell === undefined) return undefined;
  if (typeof cell === "string") return cell;
  if (typeof cell === "number" || typeof cell === "boolean") return String(cell);
  return JSON.stringify(cell);
}
