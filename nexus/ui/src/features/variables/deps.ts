// Variable dependency analysis: which variables a `query` variable's SQL
// references, the topological resolution order, and cycle detection (WS-02
// item 6). A query variable's option SQL may interpolate another variable
// (`$parent` / `${parent}` / `$__sqlIn(parent)`); that makes `parent` a
// dependency that must resolve first, and a cycle among such references is
// an authoring error we reject with a clear message rather than loop on.

import type { ResolvedVariable, VariableKind } from "@/data/types";
import { parseKindConfig } from "@/features/variables/config";

/** A variable definition as the dependency pass needs it: name, kind, and
 *  the opaque config (only `query` configs contribute edges). */
export interface VarDef {
  name: string;
  kind: VariableKind;
  optionsConfig: unknown;
}

/** Match `$name`, `${name}`, `${name:fmt}`, `$__sqlIn(name)` references to
 *  *other* variables inside a SQL string. Built-ins (`$__from`, `$__to`,
 *  `$__dashboard`, `$__user`, `$__interval`, `$__timeFilter`, …) start with
 *  `__` and are deliberately excluded — they are resolved by the time layer
 *  / server binder, not by another dashboard variable. */
const REF = /\$__sqlIn\(\s*([a-zA-Z][a-zA-Z0-9_]*)\s*\)|\$\{\s*([a-zA-Z][a-zA-Z0-9_]*)(?::[^}]*)?\s*\}|\$([a-zA-Z][a-zA-Z0-9_]*)/g;

/** The set of variable names a SQL string references (deduplicated). A
 *  name beginning with `__` is a built-in macro, not a variable, so it is
 *  never returned here. */
export function referencedVariables(sql: string): string[] {
  const found = new Set<string>();
  for (const m of sql.matchAll(REF)) {
    const name = m[1] ?? m[2] ?? m[3];
    if (name && !name.startsWith("__")) found.add(name);
  }
  return [...found];
}

/** The direct dependencies of one variable: a `query` variable depends on
 *  every *known* variable its SQL references; all other kinds have none.
 *  Unknown references (typos, built-ins) are dropped — they cannot create
 *  an edge to a node that does not exist. */
export function dependenciesOf(def: VarDef, known: Set<string>): string[] {
  if (def.kind !== "query") return [];
  const cfg = parseKindConfig("query", def.optionsConfig);
  if (cfg.kind !== "query") return [];
  return referencedVariables(cfg.sql).filter(
    (n) => n !== def.name && known.has(n),
  );
}

/** A cycle in the variable dependency graph: the chain of names that loops
 *  (e.g. `a → b → a`). Thrown shape is surfaced verbatim in the editor. */
export class VariableCycleError extends Error {
  readonly cycle: string[];
  constructor(cycle: string[]) {
    super(`Variable cycle detected: ${cycle.join(" → ")}`);
    this.name = "VariableCycleError";
    this.cycle = cycle;
  }
}

/** Topologically order variables so each resolves after its dependencies
 *  (item 6 / resolution order). Ties break by `sortOrder` then name for a
 *  stable, author-meaningful order. Throws {@link VariableCycleError} on a
 *  dependency cycle. The returned array is the order to resolve in. */
export function resolutionOrder(defs: ReadonlyArray<VarDef & { sortOrder?: number }>): VarDef[] {
  const known = new Set(defs.map((d) => d.name));
  const byName = new Map(defs.map((d) => [d.name, d] as const));
  const deps = new Map<string, string[]>(
    defs.map((d) => [d.name, dependenciesOf(d, known)] as const),
  );

  // Stable input order: sortOrder then name, so a tie among independent
  // variables follows the author's bar order.
  const ordered = [...defs].sort(
    (a, b) =>
      (a.sortOrder ?? 0) - (b.sortOrder ?? 0) || a.name.localeCompare(b.name),
  );

  const result: VarDef[] = [];
  const done = new Set<string>();
  // `onPath` is the current DFS recursion stack; a re-entry into a node on
  // the stack is a cycle.
  const onPath = new Set<string>();
  const stack: string[] = [];

  const visit = (name: string) => {
    if (done.has(name)) return;
    if (onPath.has(name)) {
      const from = stack.indexOf(name);
      throw new VariableCycleError([...stack.slice(from), name]);
    }
    onPath.add(name);
    stack.push(name);
    // Visit dependencies in stable order too.
    for (const dep of (deps.get(name) ?? []).slice().sort()) visit(dep);
    onPath.delete(name);
    stack.pop();
    done.add(name);
    const def = byName.get(name);
    if (def) result.push(def);
  };

  for (const d of ordered) visit(d.name);
  return result;
}

/** The variables (by name) whose option list depends — directly or
 *  transitively — on `changed`. Changing `changed`'s value must re-resolve
 *  exactly these children (item 7), no more. */
export function dependentsOf(
  changed: string,
  defs: ReadonlyArray<VarDef>,
): Set<string> {
  const known = new Set(defs.map((d) => d.name));
  // Reverse edges: dep -> [variables that depend on dep].
  const rev = new Map<string, string[]>();
  for (const d of defs) {
    for (const dep of dependenciesOf(d, known)) {
      const list = rev.get(dep) ?? [];
      list.push(d.name);
      rev.set(dep, list);
    }
  }
  const out = new Set<string>();
  const walk = (name: string) => {
    for (const child of rev.get(name) ?? []) {
      if (!out.has(child)) {
        out.add(child);
        walk(child);
      }
    }
  };
  walk(changed);
  return out;
}

/** Re-export so callers can take a {@link ResolvedVariable} list and reduce
 *  to the dep-pass shape without reaching into config internals. */
export function toVarDef(v: ResolvedVariable & { optionsConfig?: unknown }): VarDef {
  return { name: v.name, kind: v.kind, optionsConfig: v.optionsConfig };
}
