import { describe, it, expect } from "vitest";
import type { CompletionContext } from "@codemirror/autocomplete";
import { jsCompletionSource, type JsSymbol } from "./jsApi";

// Mirrors the real getApi payload: type-based scopes (a `ctx` prop's signature
// names a CtxX type whose members live under scope "ctx.x").
const SYMBOLS: JsSymbol[] = [
  { label: "in1", kind: "prop", signature: "in1: number", scope: "ctx" },
  { label: "out1", kind: "prop", signature: "out1: number", scope: "ctx" },
  { label: "log", kind: "method", signature: "log(...args: any[]): void", scope: "ctx" },
  { label: "self", kind: "prop", signature: "self: CtxComponent", scope: "ctx" },
  { label: "app", kind: "prop", signature: "app: CtxApp", scope: "ctx" },
  { label: "name", kind: "prop", signature: "name: string", scope: "ctx.component" },
  { label: "children", kind: "prop", signature: "children: CtxComponent[]", scope: "ctx.component" },
  { label: "parent", kind: "method", signature: "parent(): CtxComponent | null", scope: "ctx.component" },
  { label: "component", kind: "method", signature: "component(ref: string | number): CtxComponent | null", scope: "ctx.app" },
  { label: "read", kind: "method", signature: "read(ref: string): number", scope: "ctx.app" },
  { label: "evaluate", kind: "method", signature: "evaluate(): void", scope: "global" },
];

// Minimal CompletionContext stand-in. The source uses matchBefore/explicit and
// state.doc (for local-variable inference) — `doc` is the text up to the cursor,
// optionally prefixed with `pre` (earlier lines, e.g. declarations).
function mkCtx(doc: string, pre = "", explicit = true): CompletionContext {
  const full = pre + doc;
  return {
    pos: full.length,
    explicit,
    state: { doc: { toString: () => full } },
    matchBefore(re: RegExp) {
      const m = doc.match(new RegExp(re.source.replace(/\$$/, "") + "$"));
      if (!m) return null;
      const text = m[0];
      return { from: full.length - text.length, to: full.length, text };
    },
  } as unknown as CompletionContext;
}

const labels = (doc: string, pre = ""): string[] | null => {
  const r = jsCompletionSource({ current: SYMBOLS })(mkCtx(doc, pre));
  return r ? r.options.map((o) => o.label) : null;
};

describe("jsCompletionSource", () => {
  it("ctx. → ctx members", () => {
    expect(labels("ctx.")).toEqual(expect.arrayContaining(["in1", "out1", "log", "self", "app"]));
  });
  it("ctx.self. → component members (resolved via the self: CtxComponent type)", () => {
    expect(labels("ctx.self.")?.sort()).toEqual(["children", "name", "parent"]);
  });
  it("ctx.app. → app members (resolved via app: CtxApp)", () => {
    expect(labels("ctx.app.")?.sort()).toEqual(["component", "read"]);
  });
  it("follows a method-call return type (ctx.self.parent().)", () => {
    expect(labels("ctx.self.parent().")?.sort()).toEqual(["children", "name", "parent"]);
  });
  it("follows a call with args (ctx.app.component(100006).)", () => {
    expect(labels("ctx.app.component(100006).")?.sort()).toEqual(["children", "name", "parent"]);
  });
  it("infers a local var from its declaration (const c = ctx.app.component(0); c.)", () => {
    expect(labels("c.", "const c = ctx.app.component(100006);\n")?.sort()).toEqual(["children", "name", "parent"]);
  });
  it("infers a local var from a call chain (const p = ctx.self.parent(); p.)", () => {
    expect(labels("p.", "const p = ctx.self.parent();\n")?.sort()).toEqual(["children", "name", "parent"]);
  });
  it("bare identifier → ctx + globals", () => {
    expect(labels("ev")).toEqual(expect.arrayContaining(["ctx", "evaluate"]));
  });
  it("unknown object → no completions", () => {
    expect(labels("foo.")).toBeNull();
  });
});
