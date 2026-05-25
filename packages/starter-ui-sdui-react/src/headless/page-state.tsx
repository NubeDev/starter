// `PageStateContext` — the writable bag widgets (toggle, slider,
// date_range, select) push into. Re-runs `useSduiResolve` on every
// change so server-bound `$page.*` expressions reflow.

import { createContext, useContext, useMemo, useState } from "react";
import type { ReactNode } from "react";

export type PageState = Record<string, unknown>;
export type SetPageState = (next: PageState) => void;

const PageStateContext = createContext<
  [PageState, SetPageState] | null
>(null);

export function PageStateProvider({
  initial,
  children,
}: {
  initial?: PageState;
  children: ReactNode;
}) {
  const [state, setState] = useState<PageState>(initial ?? {});
  const value = useMemo<[PageState, SetPageState]>(
    () => [state, setState],
    [state],
  );
  return (
    <PageStateContext.Provider value={value}>
      {children}
    </PageStateContext.Provider>
  );
}

export function usePageState(): [PageState, SetPageState] {
  const ctx = useContext(PageStateContext);
  if (!ctx) {
    throw new Error("usePageState: missing <PageStateProvider>");
  }
  return ctx;
}

/** Write a single key/value into the page-state bag. */
export function usePageStateKey(key: string): [unknown, (value: unknown) => void] {
  const [state, setState] = usePageState();
  const setter = (value: unknown) => setState({ ...state, [key]: value });
  return [state[key], setter];
}
