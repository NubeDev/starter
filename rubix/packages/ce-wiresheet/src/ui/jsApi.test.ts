import { describe, it, expect } from "vitest";
import type { CompletionContext } from "@codemirror/autocomplete";
import { jsCompletionSource, type JsSymbol } from "./jsApi";

// Mirrors the real getApi payload: ctx members, a nested ctx.app, and globals.
const SYMBOLS: JsSymbol[] = [
  { label: "in1", kind: "prop", scope: "ctx", detail: "readonly number (input)" },
  { label: "out1", kind: "prop", scope: "ctx" },
  { label: "log", kind: "method", scope: "ctx" },
  { label: "read", kind: "method", scope: "ctx.app" },
  { label: "exists", kind: "method", scope: "ctx.app" },
  { label: "evaluate", kind: "method", scope: "global" },
];

// Minimal CompletionContext stand-in — the source only uses matchBefore/pos/explicit.
function mkCtx(doc: string, explicit = true): CompletionContext {
  return {
    pos: doc.length,
    explicit,
    matchBefore(re: RegExp) {
      const m = doc.match(new RegExp(re.source.replace(/\$$/, "") + "$"));
      if (!m) return null;
      const text = m[0];
      return { from: doc.length - text.length, to: doc.length, text };
    },
  } as unknown as CompletionContext;
}

const labels = (doc: string): string[] | null => {
  const r = jsCompletionSource({ current: SYMBOLS })(mkCtx(doc));
  return r ? r.options.map((o) => o.label) : null;
};

describe("jsCompletionSource", () => {
  it("ctx. → ctx members + derived nested namespace", () => {
    expect(labels("ctx.")).toEqual(expect.arrayContaining(["in1", "out1", "log", "app"]));
  });
  it("ctx.app. → the app members (dotted scope)", () => {
    expect(labels("ctx.app.")?.sort()).toEqual(["exists", "read"]);
  });
  it("bare identifier → ctx + globals", () => {
    expect(labels("ev")).toEqual(expect.arrayContaining(["ctx", "evaluate"]));
  });
  it("unknown object → no completions", () => {
    expect(labels("foo.")).toBeNull();
  });
});
