/**
 * `SduiRenderPage` — the pre-resolved variant of `SduiPage`. The
 * caller supplies a `UiComponentTree` directly (e.g. AI-builder
 * preview, static fixture, blog embed) along with a write plan and
 * an action dispatcher; the page does not call `/ui/resolve` and
 * does not subscribe to live updates.
 *
 * Same capability handshake (R2): a tree whose `ir_version` exceeds
 * `SUPPORTED_IR_VERSION` renders a mismatch banner instead of
 * projecting.
 */
import { useMemo, useState } from "react";
import { SduiProvider, globalCustomRegistry } from "./context.js";
import { Renderer } from "./Renderer.js";
import { checkIrVersion } from "./capability.js";
import type {
  UiActionResponse,
  UiComponentTree,
  WritePlanEntry,
} from "./types.js";
import { SduiDialogHost } from "./SduiDialogHost.js";

export interface SduiRenderPageProps {
  tree: UiComponentTree;
  writePlan?: WritePlanEntry[];
  /**
   * Optional action dispatcher. When omitted, action-bearing
   * components dispatch into a no-op resolver that returns
   * `{ type: "noop" }` — useful for static previews where the
   * tree is decorative.
   */
  dispatchAction?: (handler: string, args?: unknown) => Promise<UiActionResponse>;
}

const NOOP_DISPATCH = async (): Promise<UiActionResponse> => ({ type: "noop" });

export function SduiRenderPage({
  tree,
  writePlan,
  dispatchAction,
}: SduiRenderPageProps) {
  const [pageState, setPageState] = useState<Record<string, unknown>>({});

  const mergePageState = useMemo(
    () => (patch: Record<string, unknown>) =>
      setPageState((prev) => ({ ...prev, ...patch })),
    [],
  );

  // Pre-resolved trees do not round-trip through React-Query, so
  // there is no cache key to mutate. `treeQueryKey` is a stable
  // sentinel — optimistic patches and subscription updates that
  // assume the cache write will be no-ops, which is the intended
  // behaviour for a static preview.
  const treeQueryKey = useMemo(() => ["sdui-render-page"] as const, []);

  const mismatch = checkIrVersion(tree);
  if (mismatch) {
    return (
      <div className="p-6">
        <p className="text-sm text-destructive">
          Capability mismatch: tree has{" "}
          <code>ir_version={mismatch.received}</code>, client supports up to{" "}
          <code>{mismatch.supported}</code>.
        </p>
      </div>
    );
  }

  return (
    <SduiProvider
      dispatchAction={dispatchAction ?? NOOP_DISPATCH}
      customRegistry={globalCustomRegistry}
      pageState={pageState}
      setPageState={mergePageState}
      treeQueryKey={treeQueryKey}
      writePlan={writePlan ?? []}
    >
      <Renderer node={tree.root} />
      <SduiDialogHost />
    </SduiProvider>
  );
}
