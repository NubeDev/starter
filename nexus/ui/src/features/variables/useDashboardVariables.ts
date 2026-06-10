import { useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import type { StarterClient } from "@nube/starter-client-ts";

import { listVariables } from "@/api/variables/list";
import type { VariableDetail } from "@/api/types";
import type { PageContext, ResolvedVariable } from "@/data/types";
import { EMPTY_PAGE_CONTEXT } from "@/features/variables/context";
import {
  resolutionOrder,
  type VarDef,
  type VariableCycleError,
} from "@/features/variables/deps";
import { resolveOptions, type ResolvedSelections } from "@/features/variables/resolve";
import { useVariableStore } from "@/store/variables";

export const variablesKey = (slug: string) =>
  ["nexus", "variables", slug] as const;

/** Pick the effective selection for a variable: a URL/store override if
 *  present, else the stored `current`, else (for a list-bearing kind) the
 *  first option / "All". Always returns at least the empty array. */
function effectiveSelection(
  def: VariableDetail,
  override: ReadonlyArray<string> | undefined,
  options: ReadonlyArray<{ value: string }>,
): string[] {
  if (override && override.length > 0) return [...override];
  const stored = def.current ?? [];
  if (stored.length > 0) {
    // Keep only stored values still present in the resolved options (a
    // parent change can invalidate a child's old pick); fall back below if
    // nothing survives.
    const valid = options.length
      ? stored.filter((v) => options.some((o) => o.value === v))
      : stored;
    if (valid.length > 0) return valid;
  }
  if (def.include_all) return options.map((o) => o.value);
  return options.length > 0 ? [options[0].value] : [];
}

/** The context-derived default for a variable, by the §1 cross-source
 *  precedence (later wins): dashboard tag → nav `values` → bare URL param,
 *  matched on the variable's own name. Returns a single-value selection, or
 *  `undefined` when no source carries the name (the variable then falls back to
 *  its stored/first-option default). The bar selection, which outranks all of
 *  these, is applied by the caller. */
function contextSeed(
  name: string,
  ctx: PageContext,
): ReadonlyArray<string> | undefined {
  // URL is highest of the context sources (a deep link is the most explicit
  // external intent short of a bar pick), then nav.values, then the tag.
  const url = ctx.url[name];
  if (url !== undefined) return Array.isArray(url) ? [...url] : [url];
  const value = ctx.values[name];
  if (value !== undefined) return Array.isArray(value) ? [...value] : [value];
  const tag = ctx.tags[name];
  if (tag != null) return [tag];
  return undefined;
}

/** Resolve every variable for a dashboard in dependency order, threading
 *  each resolved selection into the next so a cascading `query` variable
 *  sees its parents' current values (item 6). Cycles surface as the thrown
 *  {@link VariableCycleError}. */
async function resolveAll(
  client: StarterClient,
  defs: VariableDetail[],
  overrides: Record<string, ReadonlyArray<string>>,
  pageContext: PageContext,
): Promise<ResolvedVariable[]> {
  const order = resolutionOrder(
    defs.map<VarDef & { sortOrder?: number }>((d) => ({
      name: d.name,
      kind: d.kind,
      optionsConfig: d.options_config,
      sortOrder: d.sort_order ?? 0,
    })),
  );
  const byName = new Map(defs.map((d) => [d.name, d] as const));

  const selections: ResolvedSelections = {};
  const resolved: ResolvedVariable[] = [];
  for (const node of order) {
    const def = byName.get(node.name);
    if (!def) continue;
    const options = await resolveOptions(
      client,
      def.kind,
      def.options_config,
      selections,
      pageContext,
    );
    // Context precedence (WS-13 §1, later wins): dashboard tags → nav.values →
    // URL bare param → explicit bar selection. The bar selection is `overrides`
    // (URL `var-*` + store); the lower-precedence context sources seed a default
    // when the bar has not overridden this variable, threaded as an override so
    // it flows the normal WS-02 selection path and bumps one revision.
    const contextDefault = contextSeed(def.name, pageContext);
    const barOverride = overrides[def.name];
    const effectiveOverride =
      barOverride && barOverride.length > 0 ? barOverride : contextDefault;
    const current = effectiveSelection(def, effectiveOverride, options);
    selections[def.name] = current;
    resolved.push({
      id: def.id,
      name: def.name,
      label: def.label ?? undefined,
      kind: def.kind,
      options,
      optionsConfig: def.options_config,
      current,
      multi: def.multi ?? false,
      includeAll: def.include_all ?? false,
      hidden: def.hidden ?? false,
      sortOrder: def.sort_order ?? 0,
    });
  }
  // Restore the author's bar order for display (resolution order is
  // dependency-topological, which can differ from sortOrder).
  resolved.sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name));
  return resolved;
}

/** Load and resolve a dashboard's variables, publishing the resolved set
 *  into the variable store so the bar and panel queries read one source of
 *  truth. Re-resolves when the slug changes or a selection bumps the
 *  store's `revision` (so a parent change re-resolves its children, item 7).
 *  A dependency cycle surfaces as the query's `error`. */
export function useDashboardVariables(
  slug: string | undefined,
  pageContext: PageContext = EMPTY_PAGE_CONTEXT,
): {
  isPending: boolean;
  error: Error | null;
  cycle: VariableCycleError | null;
} {
  const client = useStarterClient();
  const selections = useVariableStore((s) => s.selections);
  const setResolved = useVariableStore((s) => s.setResolved);
  const reset = useVariableStore((s) => s.reset);

  // Snapshot the selections + context used for this resolution pass. Reading
  // the live objects inside `queryFn` is fine since the key includes both; we
  // capture them so the dependency is explicit.
  const overridesRef = useRef(selections);
  overridesRef.current = selections;
  const contextRef = useRef(pageContext);
  contextRef.current = pageContext;

  // The assembled context is part of the resolution key (WS-13 §5): the slug is
  // unchanged when only the nav node changes, so without this two mounts of one
  // page would resolve cascading `query` options from each other's stale cache.
  const contextKey = JSON.stringify(pageContext);

  const query = useQuery({
    queryKey: [...variablesKey(slug ?? ""), JSON.stringify(selections), contextKey],
    enabled: !!slug,
    queryFn: async () => {
      const defs = await listVariables(client, slug!);
      return resolveAll(client, defs, overridesRef.current, contextRef.current);
    },
  });

  // Publish into the store on success; clear on slug change/unmount so one
  // dashboard's variables never leak into the next.
  useEffect(() => {
    if (query.data) setResolved(query.data);
  }, [query.data, setResolved]);
  useEffect(() => {
    return () => reset();
  }, [slug, reset]);

  const cycle =
    query.error && query.error.name === "VariableCycleError"
      ? (query.error as VariableCycleError)
      : null;
  return {
    isPending: query.isPending,
    error: query.error instanceof Error ? query.error : null,
    cycle,
  };
}
