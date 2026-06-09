import { useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import type { StarterClient } from "@nube/starter-client-ts";

import { listVariables } from "@/api/variables/list";
import type { VariableDetail } from "@/api/types";
import type { ResolvedVariable } from "@/data/types";
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

/** Resolve every variable for a dashboard in dependency order, threading
 *  each resolved selection into the next so a cascading `query` variable
 *  sees its parents' current values (item 6). Cycles surface as the thrown
 *  {@link VariableCycleError}. */
async function resolveAll(
  client: StarterClient,
  defs: VariableDetail[],
  overrides: Record<string, ReadonlyArray<string>>,
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
    );
    const current = effectiveSelection(def, overrides[def.name], options);
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
export function useDashboardVariables(slug: string | undefined): {
  isPending: boolean;
  error: Error | null;
  cycle: VariableCycleError | null;
} {
  const client = useStarterClient();
  const selections = useVariableStore((s) => s.selections);
  const setResolved = useVariableStore((s) => s.setResolved);
  const reset = useVariableStore((s) => s.reset);

  // Snapshot the selections used as overrides for this resolution pass.
  // Reading the live object inside `queryFn` is fine since the key includes
  // the revision; we capture it so the dependency is explicit.
  const overridesRef = useRef(selections);
  overridesRef.current = selections;

  const query = useQuery({
    queryKey: [...variablesKey(slug ?? ""), JSON.stringify(selections)],
    enabled: !!slug,
    queryFn: async () => {
      const defs = await listVariables(client, slug!);
      return resolveAll(client, defs, overridesRef.current);
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
