/**
 * R7 smoke — "Custom renderer falls back cleanly".
 *
 * Mounts the `customSpec.Component` directly (not via the full
 * `Renderer`, which transitively pulls in shadcn primitives behind
 * a Vite-only `@/...` path alias) and asserts the fallback contract:
 *
 * 1. An unknown `renderer_id` renders the neutral stub; a sibling
 *    node under the same provider renders normally.
 * 2. A registered renderer for that id replaces the stub.
 * 3. A missing `renderer_id` renders the stub with a sentinel marker.
 *
 * The renderer-side `useEffect` that emits the structured
 * `sdui.custom.unknown_renderer` warning is best-effort under SSR
 * (effects do not run during `renderToStaticMarkup`). The
 * client-side once-per-id contract is exercised by the host app's
 * `@testing-library/react` integration; this file pins the markup
 * contract that R7 depends on.
 */
import { describe, it, expect, beforeEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";

import { customSpec, __resetCustomWarningCacheForTests } from "./Custom.js";
import type { CustomNode } from "./Custom.js";
import {
  SduiProvider,
  globalCustomRegistry,
  registerCustomRenderer,
} from "../context.js";
import type { UiComponent } from "../types.js";

const Custom = customSpec.Component;

function harness(...nodes: React.ReactNode[]): string {
  return renderToStaticMarkup(
    <SduiProvider
      dispatchAction={async () => ({ type: "noop" })}
      customRegistry={globalCustomRegistry}
      pageState={{}}
      setPageState={() => {}}
      treeQueryKey={["test"]}
      writePlan={[]}
    >
      <>{nodes}</>
    </SduiProvider>,
  );
}

describe("custom escape hatch (R7)", () => {
  beforeEach(() => {
    globalCustomRegistry.clear();
    __resetCustomWarningCacheForTests();
  });

  it("unknown renderer_id renders the stub; sibling renders normally", () => {
    const unknown = {
      type: "custom",
      id: "c1",
      renderer_id: "unknown.id",
      props: { foo: 1 },
    } as unknown as CustomNode;

    const html = harness(
      <span data-sibling="1" key="s">hello sibling</span>,
      <Custom node={unknown} key="c" />,
    );

    expect(html).toContain('data-sdui-custom-stub="unknown.id"');
    expect(html).toContain("Unknown custom renderer: unknown.id");
    expect(html).toContain("hello sibling");
  });

  it("registered renderer replaces the stub", () => {
    registerCustomRenderer("known.id", ({ props }) => (
      <span data-known="1">known:{JSON.stringify(props)}</span>
    ));

    const node = {
      type: "custom",
      id: "c2",
      renderer_id: "known.id",
      props: { ok: true },
    } as unknown as CustomNode;

    const html = harness(<Custom node={node} key="c" />);
    expect(html).toContain("known:");
    expect(html).toContain("ok");
    expect(html).not.toContain("data-sdui-custom-stub");
  });

  it("missing renderer_id falls back to the stub with a sentinel marker", () => {
    const node = {
      type: "custom",
      id: "c3",
    } as unknown as CustomNode;
    const html = harness(<Custom node={node} key="c" />);
    expect(html).toContain("data-sdui-custom-stub");
    expect(html).toContain("missing");
  });
});
