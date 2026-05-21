/**
 * `useBoundWrite` — two-way binding hook for controls (slider,
 * toggle, select, text-field, etc.). The control:
 *
 *   1. Looks up its entry in `writePlan` by `component_id`. A
 *      missing entry means the binding failed to resolve (ACL,
 *      invalid path) — the control renders disabled.
 *   2. Calls `dispatchAction(handler, { target_node_id, slot, field?,
 *      value })` on commit, going through the same
 *      `POST /api/v1/ui/action` endpoint as every other interaction
 *      (R5 — one endpoint).
 *   3. Optionally writes the new value into the cached tree
 *      optimistically; the authoritative value lands either
 *      through the action's `patch` response or through the
 *      subscription update for the same slot.
 *
 * Exposed as a hook so any custom renderer that wants two-way
 * binding can opt in without re-implementing the lookup +
 * optimistic-patch dance.
 */
import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useSdui } from "./context.js";
import { mergeAt } from "./applyPatch.js";
import type { UiActionResponse, UiComponentTree, WritePlanEntry } from "./types.js";

export interface BoundWrite {
  /** `null` when the binding failed to resolve (control disabled). */
  entry: WritePlanEntry | null;
  /** Submit a new value. Returns the action response so callers can
   *  surface diagnostics or chain further effects. */
  write: (value: unknown) => Promise<UiActionResponse | null>;
}

export function useBoundWrite(componentId: string | undefined): BoundWrite {
  const { writePlan, dispatchAction, treeQueryKey } = useSdui();
  const qc = useQueryClient();

  const entry =
    componentId !== undefined
      ? writePlan.find((e) => e.component_id === componentId) ?? null
      : null;

  const write = useCallback(
    async (value: unknown): Promise<UiActionResponse | null> => {
      if (!entry || !componentId) return null;

      // Optimistic patch — write the new value into the cached
      // component before the round-trip lands. The server's
      // authoritative `patch` / subscription update overrides on
      // arrival.
      qc.setQueryData<unknown>(treeQueryKey, (prev: unknown) => {
        const tree = readTree(prev);
        if (!tree) return prev;
        const fields: Record<string, unknown> = entry.field
          ? { [entry.field]: value }
          : { value };
        return writeTree(prev, mergeAt(tree, componentId, fields));
      });

      const args = {
        target_node_id: entry.target_node_id,
        slot: entry.slot,
        field: entry.field,
        value,
      };
      return dispatchAction(entry.handler, args);
    },
    [entry, componentId, dispatchAction, qc, treeQueryKey],
  );

  return { entry, write };
}

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
