/**
 * Phase 8 — `Action.optimistic` round-trip smoke (SCOPE.md § R9).
 *
 * Pins the four contracted behaviours of
 * [`runOptimisticDispatch`]:
 *
 *   1. The optimistic patch is applied to the React-Query cache
 *      **before** the round-trip resolves.
 *   2. An authoritative `patch` reply replaces the cached value
 *      through the same `applyPatch` helpers.
 *   3. A `diagnostics` reply rolls back to the pre-dispatch
 *      snapshot.
 *   4. A thrown dispatch error rolls back to the pre-dispatch
 *      snapshot and re-throws.
 */
import { describe, it, expect, vi } from "vitest";
import { QueryClient } from "@tanstack/react-query";
import { runOptimisticDispatch } from "./useOptimisticAction.js";
import { useActionResponse } from "./useActionResponse.js";
import type {
  UiActionResponse,
  UiComponent,
  UiComponentTree,
} from "./types.js";

function seedTree(): UiComponentTree {
  return {
    ir_version: 5,
    root: {
      type: "page",
      id: "p",
      children: [
        { type: "badge", id: "b1", label: "off" } as UiComponent,
      ],
    } as UiComponent,
  };
}

function findById(
  node: UiComponent,
  id: string,
): UiComponent | null {
  if (node.id === id) return node;
  for (const c of node.children ?? []) {
    const hit = findById(c, id);
    if (hit) return hit;
  }
  return null;
}

// A lightweight `interpret` stand-in for tests — the real one is a
// React hook (useActionResponse) which needs a provider, so we
// rebuild only the `patch` / `full_render` behaviour here. The
// shared interpreter is exercised by other tests; this file is
// scoped to the optimistic / rollback contract.
function makeInterpret(qc: QueryClient, key: readonly unknown[]) {
  return function interpret(res: UiActionResponse): void {
    if (res.type === "patch") {
      const prev = qc.getQueryData<UiComponentTree>(key);
      if (!prev) return;
      qc.setQueryData<UiComponentTree>(key, {
        ...prev,
        root: mergeRecursive(prev.root, res.target_id, res.fields),
      });
    }
    if (res.type === "full_render") {
      qc.setQueryData<UiComponentTree>(key, res.render);
    }
  };
}

function mergeRecursive(
  node: UiComponent,
  id: string,
  fields: Record<string, unknown>,
): UiComponent {
  if (node.id === id) return { ...node, ...fields };
  const ch = node.children;
  if (!Array.isArray(ch)) return node;
  return { ...node, children: ch.map((c) => mergeRecursive(c, id, fields)) };
}

describe("optimistic action hints (R9 / Phase 8)", () => {
  it("applies the optimistic patch before the round-trip resolves", async () => {
    const qc = new QueryClient();
    const key = ["sdui-resolve", "page-1"] as const;
    qc.setQueryData(key, seedTree());

    let observedBeforeResolve: string | undefined;
    const dispatch = vi.fn(async (): Promise<UiActionResponse> => {
      // Inspect the cache as the server is "thinking".
      const t = qc.getQueryData<UiComponentTree>(key);
      observedBeforeResolve = (findById(t!.root, "b1")?.label ?? "") as string;
      return { type: "noop" };
    });

    await runOptimisticDispatch(
      {
        qc,
        treeQueryKey: key,
        dispatchAction: dispatch,
        interpret: makeInterpret(qc, key),
      },
      "device.toggle",
      { id: "b1" },
      { target_component_id: "b1", fields: { label: "on" } },
    );

    expect(observedBeforeResolve).toBe("on");
    // Post-noop: the optimistic state remains the authoritative
    // state (SCOPE: "either it confirms (no-op) or it returns an
    // authoritative Patch/FullRender that replaces").
    const after = qc.getQueryData<UiComponentTree>(key);
    expect(findById(after!.root, "b1")?.label).toBe("on");
  });

  it("authoritative patch reply replaces through the same applyPatch helpers", async () => {
    const qc = new QueryClient();
    const key = ["sdui-resolve", "page-2"] as const;
    qc.setQueryData(key, seedTree());

    const dispatch = vi.fn(async (): Promise<UiActionResponse> => ({
      type: "patch",
      target_id: "b1",
      fields: { label: "AUTH" },
    }));

    await runOptimisticDispatch(
      {
        qc,
        treeQueryKey: key,
        dispatchAction: dispatch,
        interpret: makeInterpret(qc, key),
      },
      "device.toggle",
      null,
      { target_component_id: "b1", fields: { label: "OPT" } },
    );

    const after = qc.getQueryData<UiComponentTree>(key);
    expect(findById(after!.root, "b1")?.label).toBe("AUTH");
  });

  it("rolls back on dispatch throw and re-throws", async () => {
    const qc = new QueryClient();
    const key = ["sdui-resolve", "page-3"] as const;
    qc.setQueryData(key, seedTree());

    const dispatch = vi.fn(async (): Promise<UiActionResponse> => {
      throw new Error("boom");
    });

    await expect(
      runOptimisticDispatch(
        {
          qc,
          treeQueryKey: key,
          dispatchAction: dispatch,
          interpret: makeInterpret(qc, key),
        },
        "device.toggle",
        null,
        { target_component_id: "b1", fields: { label: "OPT" } },
      ),
    ).rejects.toThrow(/boom/);

    const after = qc.getQueryData<UiComponentTree>(key);
    expect(findById(after!.root, "b1")?.label).toBe("off");
  });

  it("diagnostics reply rolls back the optimistic write", async () => {
    const qc = new QueryClient();
    const key = ["sdui-resolve", "page-4"] as const;
    qc.setQueryData(key, seedTree());

    const dispatch = vi.fn(async (): Promise<UiActionResponse> => ({
      type: "diagnostics",
      items: [
        { severity: "error", code: "forbidden", message: "nope" },
      ],
    }));

    const interpretCalls: UiActionResponse[] = [];
    const interpret = (r: UiActionResponse) => {
      interpretCalls.push(r);
    };

    const res = await runOptimisticDispatch(
      { qc, treeQueryKey: key, dispatchAction: dispatch, interpret },
      "device.toggle",
      null,
      { target_component_id: "b1", fields: { label: "OPT" } },
    );

    expect(res.type).toBe("diagnostics");
    expect(interpretCalls).toHaveLength(1);
    const after = qc.getQueryData<UiComponentTree>(key);
    expect(findById(after!.root, "b1")?.label).toBe("off");
  });

  // Sanity — the real `useActionResponse` hook (not invoked here)
  // is the canonical interpreter; we re-export it so this import
  // exercises that the hook surface is still wired.
  it("re-exports useActionResponse", () => {
    expect(typeof useActionResponse).toBe("function");
  });
});
