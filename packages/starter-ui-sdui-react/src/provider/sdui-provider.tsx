// `<SduiProvider>` — wires the transport into React context so the
// `useSdui*` hooks find it. Consumers wrap their dashboard subtree
// once; everything underneath shares the same transport instance.

import { createContext, useContext } from "react";
import type { ReactNode } from "react";
import type { SduiTransport } from "../transport/index.js";
import type { CustomRendererRegistry } from "../renderer/registry.js";

export interface SduiContextValue {
  transport: SduiTransport;
  /**
   * Optional registry for custom renderers — rubix-shipped widgets
   * (e.g. `rubix.alarm-table`) that this package doesn't know about.
   * Looked up by `<RenderCustom>` only.
   */
  customRenderers?: CustomRendererRegistry;
}

const SduiContext = createContext<SduiContextValue | null>(null);

export interface SduiProviderProps extends SduiContextValue {
  children: ReactNode;
}

export function SduiProvider({
  children,
  transport,
  customRenderers,
}: SduiProviderProps) {
  return (
    <SduiContext.Provider value={{ transport, customRenderers }}>
      {children}
    </SduiContext.Provider>
  );
}

export function useSduiContext(): SduiContextValue {
  const ctx = useContext(SduiContext);
  if (!ctx) {
    throw new Error(
      "useSdui*: missing <SduiProvider>. Wrap the subtree in <SduiProvider transport={...} />.",
    );
  }
  return ctx;
}

export function useSduiTransport(): SduiTransport {
  return useSduiContext().transport;
}
