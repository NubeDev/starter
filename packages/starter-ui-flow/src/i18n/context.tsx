// React context that carries the resolved `FlowMessages` from
// `<FlowCanvas>` down to whatever renders inside it (BaseNode,
// custom node components, palette, …).
//
// Kept tiny on purpose: the package never reaches into a translation
// hook, so the context just memoises a fully-merged
// `FlowMessages` object the host derives once per locale change.

import { createContext, useContext, useMemo, type ReactNode } from "react";
import {
  DEFAULT_FLOW_MESSAGES,
  mergeFlowMessages,
  type FlowMessages,
} from "./messages.js";

const FlowI18nContext = createContext<FlowMessages>(DEFAULT_FLOW_MESSAGES);

export interface FlowI18nProviderProps {
  /** Partial override merged on top of `DEFAULT_FLOW_MESSAGES`. */
  value?: Partial<FlowMessages>;
  children: ReactNode;
}

/** Provider — `<FlowCanvas>` wraps its tree in this automatically. */
export function FlowI18nProvider({ value, children }: FlowI18nProviderProps) {
  const merged = useMemo(() => mergeFlowMessages(value), [value]);
  return (
    <FlowI18nContext.Provider value={merged}>
      {children}
    </FlowI18nContext.Provider>
  );
}

/** Read the current `FlowMessages`. Falls back to defaults outside a
 * `<FlowCanvas>` / `<FlowI18nProvider>`. */
export function useFlowMessages(): FlowMessages {
  return useContext(FlowI18nContext);
}
