/**
 * SDUI renderer context — holds the action dispatcher, the custom
 * renderer registry, the page-state writer, the owning React-Query
 * key, and the write plan emitted by the resolver, so every node in
 * the tree can reach them without prop-drilling.
 *
 * `SduiProvider` is the public entry; `useSdui` is the consumer hook.
 */
import React, { createContext, useContext } from "react";
import type { UiActionResponse, WritePlanEntry } from "./types.js";

export type ActionFn = (
  handler: string,
  args?: unknown,
) => Promise<UiActionResponse>;

export type CustomRegistry = Map<
  string,
  React.ComponentType<{ props: unknown; subscribe: string[] }>
>;

/**
 * Shared module-level custom-renderer registry. Plugins (or the host
 * bootstrap) populate this at load time; `SduiPage` and
 * `SduiRenderPage` both reference this single instance so any
 * registration is visible to every tree.
 */
export const globalCustomRegistry: CustomRegistry = new Map();

/**
 * Public registration entry — the canonical way for consumers to add
 * a custom renderer to the registry that `SduiProvider` ships with.
 * Returns an unregister function for cleanup in dev / hot-reload.
 */
export function registerCustomRenderer(
  kind: string,
  component: React.ComponentType<{ props: unknown; subscribe: string[] }>,
): () => void {
  globalCustomRegistry.set(kind, component);
  return () => {
    if (globalCustomRegistry.get(kind) === component) {
      globalCustomRegistry.delete(kind);
    }
  };
}

export interface SduiCtx {
  dispatchAction: ActionFn;
  customRegistry: CustomRegistry;
  /** Page-local state: read in children, written via setPageState in the page root. */
  pageState: Record<string, unknown>;
  setPageState: (patch: Record<string, unknown>) => void;
  /**
   * React-Query key of the owning `/ui/resolve` / `/ui/render`
   * response. Optimistic action hints and authoritative Patch /
   * FullRender responses use this to write back through
   * `queryClient.setQueryData(...)`.
   */
  treeQueryKey: readonly unknown[];
  /**
   * Write plan emitted by the resolver — one entry per two-way bound
   * control. Look up by `component_id` before writing. A missing
   * entry means the control renders disabled (ACL-denied / binding
   * resolution error).
   */
  writePlan: WritePlanEntry[];
}

const SduiContext = createContext<SduiCtx | null>(null);

export function useSdui(): SduiCtx {
  const ctx = useContext(SduiContext);
  if (!ctx) throw new Error("useSdui must be used inside <SduiProvider>");
  return ctx;
}

export function SduiProvider({
  dispatchAction,
  customRegistry,
  pageState,
  setPageState,
  treeQueryKey,
  writePlan,
  children,
}: SduiCtx & { children: React.ReactNode }) {
  return (
    <SduiContext.Provider
      value={{
        dispatchAction,
        customRegistry,
        pageState,
        setPageState,
        treeQueryKey,
        writePlan,
      }}
    >
      {children}
    </SduiContext.Provider>
  );
}
