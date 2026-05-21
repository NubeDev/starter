/**
 * `useActionResponse` — discriminator for `UiActionResponse`. Maps a
 * server-returned response variant to the right client effect:
 *
 *   - `noop`           → ignore
 *   - `toast`          → host toast hook (placeholder console.log
 *                        until starter ships a toast primitive)
 *   - `redirect`       → window.location navigation
 *   - `open_url`       → window.open
 *   - `patch`          → `mergeAt` into the cached tree under
 *                        `treeQueryKey`
 *   - `full_render`    → replace the cached tree
 *   - `dialog`         → push onto the dialog bus
 *   - `dismiss_dialog` → pop the dialog bus
 *   - `diagnostics`    → returned to the caller (forms handle their
 *                        own inline rendering); a non-form caller
 *                        falls back to a console warn
 *
 * The hook returns a single `interpret(res)` function so callers
 * (buttons, form submit handlers, table row actions) all share one
 * dispatch table — divergence drift between variants is impossible.
 */
import { useQueryClient } from "@tanstack/react-query";
import { useSdui } from "./context.js";
import { mergeAt } from "./applyPatch.js";
import { pushDialog, popDialog } from "./dialog-bus.js";
import type { UiActionResponse, UiComponentTree } from "./types.js";

export function useActionResponse() {
  const qc = useQueryClient();
  const { treeQueryKey } = useSdui();

  return function interpret(res: UiActionResponse): void {
    switch (res.type) {
      case "noop":
        return;

      case "toast":
        // Host wires a real toast in production; the renderer must
        // not depend on a specific toast library.
        // eslint-disable-next-line no-console
        console.info(`[sdui:toast:${res.intent ?? "info"}] ${res.message}`);
        return;

      case "redirect":
        window.location.href = res.href;
        return;

      case "open_url":
        window.open(res.href, res.target ?? "_blank", "noopener,noreferrer");
        return;

      case "patch":
        qc.setQueryData<unknown>(treeQueryKey, (prev: unknown) => {
          const tree = readTree(prev);
          if (!tree) return prev;
          return writeTree(prev, mergeAt(tree, res.target_id, res.fields));
        });
        return;

      case "full_render":
        qc.setQueryData<unknown>(treeQueryKey, (prev: unknown) => writeTree(prev, res.render));
        return;

      case "dialog":
        pushDialog(res.tree);
        return;

      case "dismiss_dialog":
        popDialog();
        return;

      case "diagnostics":
        // Forms intercept this variant before it reaches the
        // shared interpreter — they render diagnostics inline. A
        // non-form caller (e.g. a button) lands here; surface the
        // first error message as a console warning so the failure
        // mode is visible during development.
        // eslint-disable-next-line no-console
        console.warn("[sdui:diagnostics]", res.items);
        return;
    }
  };
}

/**
 * Resolve the underlying `UiComponentTree` from whatever React-Query
 * has cached under `treeQueryKey`. The cache may be a
 * `UiResolveResponseOk` (resolve route) or a `UiComponentTree`
 * (render route) — both shapes carry a `render` / direct tree.
 */
function readTree(cached: unknown): UiComponentTree | null {
  if (!cached || typeof cached !== "object") return null;
  if ("render" in cached) {
    const c = cached as { render?: UiComponentTree };
    return c.render ?? null;
  }
  if ("ir_version" in cached && "root" in cached) {
    return cached as UiComponentTree;
  }
  return null;
}

function writeTree(prev: unknown, tree: UiComponentTree): unknown {
  if (!prev || typeof prev !== "object") return prev;
  if ("render" in prev) {
    return { ...(prev as object), render: tree };
  }
  return tree;
}
