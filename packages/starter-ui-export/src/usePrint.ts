import { useCallback, useRef, useState } from "react";

import { printNode } from "./printNode";
import { DEFAULT_PAGE_OPTIONS, type PageOptions } from "./types";

/**
 * Return shape of {@link usePrint}.
 */
export interface UsePrintResult {
  /**
   * Pass to {@link PrintableContent} as `hostRef`. Captures the hidden
   * host element so {@link print} can hand it to {@link printNode}.
   */
  hostRef: (node: HTMLDivElement | null) => void;
  /** Open the native print dialog targeting the printable subtree. */
  print: () => Promise<void>;
  /** True while images/fonts are loading and the dialog is opening. */
  printing: boolean;
  /** Last error from {@link print}, or `null` if none. */
  error: Error | null;
}

/**
 * Wires {@link PrintableContent} to {@link printNode}.
 *
 * ```tsx
 * const { hostRef, print, printing } = usePrint();
 * return (
 *   <>
 *     <button onClick={print} disabled={printing}>Print</button>
 *     <PrintableContent hostRef={hostRef}>
 *       <MyExportView />
 *     </PrintableContent>
 *   </>
 * );
 * ```
 */
export function usePrint(
  options: PageOptions = DEFAULT_PAGE_OPTIONS,
): UsePrintResult {
  const nodeRef = useRef<HTMLDivElement | null>(null);
  const [printing, setPrinting] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const hostRef = useCallback((node: HTMLDivElement | null) => {
    nodeRef.current = node;
  }, []);

  const print = useCallback(async () => {
    setError(null);
    const node = nodeRef.current;
    if (!node) {
      setError(new Error("printable host is not mounted"));
      return;
    }
    setPrinting(true);
    try {
      await printNode(node, options);
    } catch (e) {
      setError(e instanceof Error ? e : new Error(String(e)));
    } finally {
      setPrinting(false);
    }
  }, [options]);

  return { hostRef, print, printing, error };
}
