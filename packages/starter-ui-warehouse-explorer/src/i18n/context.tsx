// React context carrying the resolved `ExplorerMessages` from
// `<Explorer>` (or a standalone `<ExplorerI18nProvider>`) down to
// the view components.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import { createContext, useContext, useMemo, type ReactNode } from "react";
import {
  DEFAULT_EXPLORER_MESSAGES,
  mergeExplorerMessages,
  type ExplorerMessages,
} from "./messages.js";

const ExplorerI18nContext = createContext<ExplorerMessages>(
  DEFAULT_EXPLORER_MESSAGES,
);

export interface ExplorerI18nProviderProps {
  /** Partial override merged on top of `DEFAULT_EXPLORER_MESSAGES`. */
  value?: Partial<ExplorerMessages>;
  children: ReactNode;
}

/** Provider — `<Explorer>` wraps its tree in this automatically. */
export function ExplorerI18nProvider({
  value,
  children,
}: ExplorerI18nProviderProps) {
  const merged = useMemo(() => mergeExplorerMessages(value), [value]);
  return (
    <ExplorerI18nContext.Provider value={merged}>
      {children}
    </ExplorerI18nContext.Provider>
  );
}

/** Read the current `ExplorerMessages`. Falls back to defaults
 * outside a provider. */
export function useExplorerMessages(): ExplorerMessages {
  return useContext(ExplorerI18nContext);
}
