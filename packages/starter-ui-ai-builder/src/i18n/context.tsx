// React context carrying the resolved `BuilderMessages` from
// `<AiBuilder>` down to the transcript + canvas. Components consume
// via `useBuilderMessages()`; callers can still pass explicit
// per-prop overrides (e.g. `<BuilderTranscript busyLabel="…" />`).

import { createContext, useContext, useMemo, type ReactNode } from "react";
import {
  DEFAULT_BUILDER_MESSAGES,
  mergeBuilderMessages,
  type BuilderMessages,
} from "./messages.js";

const BuilderI18nContext = createContext<BuilderMessages>(
  DEFAULT_BUILDER_MESSAGES,
);

export interface BuilderI18nProviderProps {
  /** Partial override merged on top of `DEFAULT_BUILDER_MESSAGES`. */
  value?: Partial<BuilderMessages>;
  children: ReactNode;
}

/** Provider — `<AiBuilder>` wraps its tree in this automatically
 * when given an `i18n` prop. Composing primitives by hand? Wrap
 * them yourself. */
export function BuilderI18nProvider({
  value,
  children,
}: BuilderI18nProviderProps) {
  const merged = useMemo(() => mergeBuilderMessages(value), [value]);
  return (
    <BuilderI18nContext.Provider value={merged}>
      {children}
    </BuilderI18nContext.Provider>
  );
}

/** Read the current `BuilderMessages`. Falls back to English defaults
 * outside any provider. */
export function useBuilderMessages(): BuilderMessages {
  return useContext(BuilderI18nContext);
}
