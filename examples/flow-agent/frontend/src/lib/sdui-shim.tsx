// `<SduiHost>` — a thin no-op wrapper around `SduiProvider` from
// `@nube/starter-sdui-react`. The Page Builder slice is fixture-
// driven and read-only, so every action handler is a no-op (SCOPE
// D3 / R4).
//
// Re-used by both the builder canvas (`PageBuilder.tsx`) and the
// read-only viewer (`PageView.tsx`) so a saved tree round-trips
// through the exact same provider it was built under.

import { useState, type ReactNode } from "react";
import {
  SduiProvider,
  globalCustomRegistry,
  type ActionFn,
} from "@nube/starter-sdui-react";

// Module-level so the reference is stable across re-renders — the
// SduiProvider treats its callbacks as plain values, but stable
// identities still help React DevTools and any downstream memoised
// consumer.
const noopDispatch: ActionFn = async (handler, args) => {
  // The Page Builder slice has no real backend; saved trees that
  // wire an action are visualised, not executed. DEV-only debug log
  // surfaces the seam without spamming production builds.
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.debug("[SduiHost] action ignored", { handler, args });
  }
  return { type: "noop" };
};

// The owning React-Query key for the resolved tree. The slice does
// not use React Query for page resolution (trees come from the
// builder hook or `localStorage`), so an empty tuple is the
// canonical "no owning query" marker.
const EMPTY_QUERY_KEY: readonly unknown[] = [];

export interface SduiHostProps {
  children: ReactNode;
}

/**
 * Mounts a `SduiProvider` configured for view-only / fixture-driven
 * rendering. Any interactive node embedded in a saved tree gets a
 * no-op action dispatcher; no real backend is contacted.
 */
export function SduiHost({ children }: SduiHostProps) {
  // Local page-state bag. Real apps wire this to a query or a store;
  // here we keep it in component state so any interactive node that
  // writes to it (e.g. a tab control) keeps working under the no-op
  // host. Resets on remount, which is fine for a viewer.
  const [pageState, setPageStateRaw] = useState<Record<string, unknown>>({});

  return (
    <SduiProvider
      dispatchAction={noopDispatch}
      customRegistry={globalCustomRegistry}
      pageState={pageState}
      setPageState={(patch) =>
        setPageStateRaw((prev) => ({ ...prev, ...patch }))
      }
      treeQueryKey={EMPTY_QUERY_KEY}
      writePlan={[]}
    >
      {children}
    </SduiProvider>
  );
}
