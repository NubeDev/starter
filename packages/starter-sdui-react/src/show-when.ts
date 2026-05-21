/**
 * Client-side predicate evaluator for `style.show_when` — gates a
 * node's mount on a tiny boolean DSL over `pageState`. Server-side
 * bindings are already flattened at resolve time (R4); this predicate
 * is the only client-evaluated gate, because it depends on
 * page-local interaction state (open tabs, filter chips, etc.) that
 * never round-trips to the server.
 */
import type { ShowWhen } from "./types.js";

export function evaluateShowWhen(
  predicate: ShowWhen,
  pageState: Record<string, unknown>,
): boolean {
  if ("all" in predicate) {
    return predicate.all.every((p) => evaluateShowWhen(p, pageState));
  }
  if ("any" in predicate) {
    return predicate.any.some((p) => evaluateShowWhen(p, pageState));
  }
  if ("not" in predicate) {
    return !evaluateShowWhen(predicate.not, pageState);
  }
  if ("eq" in predicate) {
    return readPath(predicate.eq.path, pageState) === predicate.eq.value;
  }
  if ("ne" in predicate) {
    return readPath(predicate.ne.path, pageState) !== predicate.ne.value;
  }
  if ("truthy" in predicate) {
    return !!readPath(predicate.truthy.path, pageState);
  }
  if ("falsy" in predicate) {
    return !readPath(predicate.falsy.path, pageState);
  }
  return true;
}

function readPath(path: string, state: Record<string, unknown>): unknown {
  const parts = path.split(".");
  let cursor: unknown = state;
  for (const p of parts) {
    if (cursor == null || typeof cursor !== "object") return undefined;
    cursor = (cursor as Record<string, unknown>)[p];
  }
  return cursor;
}
