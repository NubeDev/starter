// Page context (WS-13 §1) — the resolved view of a page's *place* (its nav
// node, the URL, the dashboard's tags, the node's `values` override) that a
// `context` variable reads from. This module owns reading a value out of an
// assembled `PageContext`; the assembly itself (precedence merge of the four
// sources) lands alongside the dashboard load path. Resolution is synchronous
// — a `context` variable never fetches.

import type { ContextSource, PageContext } from "@/data/types";
import type { ContextConfig } from "@/features/variables/config";

/** An empty page context — the baseline when a page is opened with no nav
 *  node and no URL params (e.g. a direct `d/:slug` hit). Every slot is
 *  present-but-empty so readers never branch on undefined. */
export const EMPTY_PAGE_CONTEXT: PageContext = {
  url: {},
  tags: {},
  values: {},
};

/** Read a single value out of the assembled context for a `context` variable.
 *  Returns the resolved string (the value the variable binds), or `undefined`
 *  when the source/key is absent — the caller then yields no option, so the
 *  variable resolves empty rather than to a stale value.
 *
 *  Multi-valued URL params collapse to their first entry: a `context` variable
 *  binds one value (use a `query` variable for a list). */
export function resolveContextValue(
  cfg: ContextConfig,
  ctx: PageContext,
): string | undefined {
  switch (cfg.source) {
    case "nav":
      return resolveNav(cfg.key, ctx);
    case "url":
      return first(ctx.url[cfg.key]);
    case "tag":
      return ctx.tags[cfg.key] ?? undefined;
    case "values":
      return first(ctx.values[cfg.key]);
  }
}

/** The `nav` source addresses the nav node itself: `slug`, `name`, or
 *  `path[n]` (the nth ancestor title, root-first). */
function resolveNav(key: string, ctx: PageContext): string | undefined {
  if (!ctx.nav) return undefined;
  if (key === "slug") return ctx.nav.slug;
  if (key === "name") return ctx.nav.name;
  const match = /^path\[(\d+)\]$/.exec(key);
  if (match) return ctx.nav.path[Number(match[1])];
  return undefined;
}

/** Built-in `$__`-namespaced context tokens (WS-13 §2) — always present, no
 *  authoring. These are resolver-side tokens (not `VariableKind`s): the
 *  variable layer exposes them alongside `$__dashboard`/`$__user`.
 *  - `$__nav_slug` / `$__nav_name` → the current nav node's slug/name
 *  - `$__tag(key)`               → the dashboard's `key` tag value
 *  Returns the resolved string, or `undefined` when unresolvable (the token
 *  then expands to empty, matching the other `$__` built-ins). */
export function resolveContextToken(
  token: string,
  ctx: PageContext,
): string | undefined {
  if (token === "__nav_slug") return ctx.nav?.slug;
  if (token === "__nav_name") return ctx.nav?.name;
  const tag = /^__tag\((.+)\)$/.exec(token);
  if (tag) return ctx.tags[tag[1]] ?? undefined;
  return undefined;
}

/** Whether `token` (without the `$`) is a built-in context token this module
 *  resolves — so the `$__` namespace owner can route it here. */
export function isContextToken(token: string): boolean {
  return (
    token === "__nav_slug" ||
    token === "__nav_name" ||
    /^__tag\(.+\)$/.test(token)
  );
}

/** Collapse a possibly-multi URL/values entry to a single bound value. */
function first(v: string | string[] | undefined): string | undefined {
  if (v === undefined) return undefined;
  return Array.isArray(v) ? v[0] : v;
}

/** Re-export for callers that want the source list (e.g. the variable form). */
export const CONTEXT_SOURCES: readonly ContextSource[] = [
  "nav",
  "url",
  "tag",
  "values",
];
