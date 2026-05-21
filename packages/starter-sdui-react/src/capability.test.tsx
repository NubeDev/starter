/**
 * R2 smoke — "Capability mismatch refuses to render".
 *
 * The capability handshake is a single function — `checkIrVersion`
 * — that returns a non-null mismatch descriptor for any tree whose
 * `ir_version` exceeds `SUPPORTED_IR_VERSION`. Both `SduiPage`
 * (resolve flow) and `SduiRenderPage` (pre-resolved flow) consult
 * it before mounting `Renderer` and render a banner instead of
 * projecting the tree.
 *
 * This test pins the contract at the function boundary and at the
 * banner-vs-mounted-renderer integration. We do not import
 * `SduiRenderPage` directly here — that would pull the full
 * Renderer dispatch table and the shadcn primitives chain (with
 * Vite-only `@/...` path aliases that vitest's node environment
 * cannot resolve, see `vitest.config.ts`). Instead, we inline the
 * exact gate `SduiRenderPage` uses (`checkIrVersion` then a
 * banner, otherwise mount), prove the function-driven branch
 * decision, and prove the renderer-side never sees the
 * mismatched tree's root.
 */
import React from "react";
import { describe, it, expect, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";

import { SUPPORTED_IR_VERSION, checkIrVersion } from "./capability.js";
import type { UiComponentTree } from "./types.js";

function tree(ir_version: number): UiComponentTree {
  return {
    ir_version,
    root: { type: "page", title: "mounted-root-sentinel", children: [] },
  } as unknown as UiComponentTree;
}

// Mirrors the gate inside `SduiPage` / `SduiRenderPage` — the
// contract we are pinning is "if `checkIrVersion` returns a
// mismatch, the banner renders and the Renderer dispatcher is
// never called with the offending tree's root."
function GateFixture({
  tree,
  onMount,
}: {
  tree: UiComponentTree;
  onMount: (root: unknown) => React.ReactNode;
}) {
  const mismatch = checkIrVersion(tree);
  if (mismatch) {
    return (
      <div data-test="capability-banner">
        capability-mismatch ir_version={mismatch.received} supported=
        {mismatch.supported}
      </div>
    );
  }
  return <>{onMount(tree.root)}</>;
}

describe("R2 — capability mismatch refuses to render", () => {
  it("returns null for an in-range tree", () => {
    expect(checkIrVersion(tree(SUPPORTED_IR_VERSION))).toBeNull();
  });

  it("returns null for an older tree (server clamped down)", () => {
    expect(checkIrVersion(tree(SUPPORTED_IR_VERSION - 1))).toBeNull();
  });

  it("returns a mismatch descriptor for a V+1 tree", () => {
    const m = checkIrVersion(tree(SUPPORTED_IR_VERSION + 1));
    expect(m).not.toBeNull();
    expect(m).toMatchObject({
      kind: "capability-mismatch",
      supported: SUPPORTED_IR_VERSION,
      received: SUPPORTED_IR_VERSION + 1,
    });
  });

  it("banner renders and the dispatcher is never called for a V+1 tree", () => {
    const dispatcher = vi.fn((_root: unknown) => (
      <div>mounted-root-sentinel</div>
    ));
    const html = renderToStaticMarkup(
      <GateFixture
        tree={tree(SUPPORTED_IR_VERSION + 7)}
        onMount={dispatcher}
      />,
    );
    expect(html).toContain("capability-banner");
    expect(html).toContain(`ir_version=${SUPPORTED_IR_VERSION + 7}`);
    expect(html).not.toContain("mounted-root-sentinel");
    expect(dispatcher).not.toHaveBeenCalled();
  });

  it("dispatcher is called when ir_version is in range", () => {
    const dispatcher = vi.fn((_root: unknown) => (
      <div>mounted-root-sentinel</div>
    ));
    const html = renderToStaticMarkup(
      <GateFixture tree={tree(SUPPORTED_IR_VERSION)} onMount={dispatcher} />,
    );
    expect(html).toContain("mounted-root-sentinel");
    expect(html).not.toContain("capability-banner");
    expect(dispatcher).toHaveBeenCalledOnce();
  });
});
