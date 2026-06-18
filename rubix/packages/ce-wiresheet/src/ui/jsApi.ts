// CodeMirror autocompletion fed by the jsScriptStore `getApi` action.
//
// getApi() → { dts, symbols } where `symbols` is a JSON array of
//   { label, kind:"prop"|"method"|…, signature, detail, doc, scope }
// `scope` groups completions: "ctx" → members offered after `ctx.`, "global"
// (or absent) → bare-identifier completions. Fetched once per service and
// cached, then read from a ref so the editor's extensions stay stable.

import type { Completion, CompletionContext, CompletionResult } from "@codemirror/autocomplete";
import type { FlexValue } from "../lib/engine-types";

export interface JsSymbol {
  label: string;
  kind?: string;
  signature?: string;
  detail?: string;
  doc?: string;
  scope?: string;
}

type CallAction = (uid: number, name: string, params?: Record<string, FlexValue>) => Promise<Record<string, FlexValue>>;

/** A labeled example script from the service's getExamples library. */
export interface JsExample {
  label: string;
  source: string;
  desc?: string;
}

const examplesCache = new Map<number, Promise<JsExample[]>>();

/** Fetch + cache the example-script library. Defensive about the item shape
 *  (label/name/title, source/code) since getExamples is still in flux. */
export function loadExamples(call: CallAction, serviceUid: number, action = "getExamples"): Promise<JsExample[]> {
  let pr = examplesCache.get(serviceUid);
  if (!pr) {
    pr = call(serviceUid, action, {}).then((ret) => {
      const raw = typeof ret?.examples === "string" ? ret.examples : "";
      try {
        const arr = JSON.parse(raw);
        if (!Array.isArray(arr)) return [];
        return arr.map((e: Record<string, unknown>, i: number): JsExample => ({
          label: String(e.label ?? e.name ?? e.title ?? `example ${i + 1}`),
          source: String(e.source ?? e.code ?? ""),
          desc: typeof (e.desc ?? e.description) === "string" ? (e.desc ?? e.description) as string : undefined,
        }));
      } catch {
        return [];
      }
    });
    examplesCache.set(serviceUid, pr);
    pr.catch(() => examplesCache.delete(serviceUid));
  }
  return pr;
}

const cache = new Map<number, Promise<JsSymbol[]>>();

/** Fetch + cache the script API symbols for a jsScriptStore (keyed by its uid). */
export function loadJsApi(call: CallAction, serviceUid: number, action = "getApi"): Promise<JsSymbol[]> {
  let pr = cache.get(serviceUid);
  if (!pr) {
    pr = call(serviceUid, action, {}).then((ret) => {
      const raw = typeof ret?.symbols === "string" ? ret.symbols : "";
      try {
        const arr = JSON.parse(raw);
        return Array.isArray(arr) ? (arr as JsSymbol[]) : [];
      } catch {
        return [];
      }
    });
    cache.set(serviceUid, pr);
    pr.catch(() => cache.delete(serviceUid)); // don't cache a failed fetch
  }
  return pr;
}

const typeOf = (k?: string): Completion["type"] =>
  k === "method" ? "method" : k === "prop" ? "property" : k === "function" ? "function" : "variable";

const toCompletion = (s: JsSymbol): Completion => ({
  label: s.label,
  type: typeOf(s.kind),
  detail: s.detail || s.signature,
  info: s.doc || s.signature,
});

/** Symbols whose scope nests one level under `parent` (e.g. parent "ctx" with a
 *  symbol scoped "ctx.app") imply an intermediate member `app` on the parent.
 *  Surface those as namespace completions so nested APIs are discoverable. */
function childNamespaces(symbols: JsSymbol[], parent: string, existing: Set<string>): Completion[] {
  const prefix = parent ? parent + "." : "";
  const segs = new Set<string>();
  for (const s of symbols) {
    if (!s.scope || !s.scope.startsWith(prefix)) continue;
    const rest = s.scope.slice(prefix.length);
    if (!rest) continue;
    const seg = rest.split(".")[0];
    if (seg && !existing.has(seg)) segs.add(seg);
  }
  return [...segs].map((label) => ({ label, type: "namespace" as const, detail: "namespace" }));
}

/** Completion source reading the latest symbols from `ref`. Member access
 *  (`a.b.c`) matches the full dotted object path against `scope`; bare
 *  identifiers offer global-scope symbols plus `ctx`. Intermediate namespaces
 *  (e.g. `ctx.app`) are derived so they appear under their parent. */
export function jsCompletionSource(ref: { current: JsSymbol[] }) {
  return (context: CompletionContext): CompletionResult | null => {
    const symbols = ref.current;
    // full dotted path before the final dot, e.g. "ctx.app" in "ctx.app.re|"
    const member = context.matchBefore(/([\w$]+(?:\.[\w$]+)*)\.([\w$]*)$/);
    if (member) {
      const lastDot = member.text.lastIndexOf(".");
      const objPath = member.text.slice(0, lastDot);
      const direct = symbols.filter((s) => s.scope === objPath).map(toCompletion);
      const ns = childNamespaces(symbols, objPath, new Set(direct.map((d) => d.label)));
      const options = [...direct, ...ns];
      if (options.length === 0) return null;
      return { from: member.from + lastDot + 1, options, validFor: /^[\w$]*$/ };
    }
    const word = context.matchBefore(/[\w$]*/);
    if (!word || (word.from === word.to && !context.explicit)) return null;
    const globals = symbols.filter((s) => !s.scope || s.scope === "global").map(toCompletion);
    const options: Completion[] = [{ label: "ctx", type: "variable", detail: "script context" }, ...globals];
    return { from: word.from, options, validFor: /^[\w$]*$/ };
  };
}
