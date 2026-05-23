import { useEffect, useState, type JSX, type ReactNode } from "react";
import { createPortal } from "react-dom";

/**
 * Props for {@link PrintableContent}.
 */
export interface PrintableContentProps {
  /** React tree to render into the hidden print host. */
  children: ReactNode;
  /**
   * Receives the host element on mount, `null` on unmount. Pair with
   * {@link usePrint} or pass directly to {@link printNode} /
   * {@link exportNodeToPdf}.
   */
  hostRef: (node: HTMLDivElement | null) => void;
}

/**
 * Renders `children` into a hidden, offscreen container outside the
 * normal layout flow so they can be handed to {@link printNode} or
 * {@link exportNodeToPdf} without disturbing the live page.
 *
 * The host is positioned at `left: -10000px` with a fixed A4-ish
 * width so layout reflects what the printed page will see. It is not
 * `display: none` — print/capture both need real layout.
 */
export function PrintableContent({
  children,
  hostRef,
}: PrintableContentProps): JSX.Element | null {
  const [host, setHost] = useState<HTMLDivElement | null>(null);

  useEffect(() => {
    const el = document.createElement("div");
    el.setAttribute("data-starter-export-host", "");
    el.style.position = "fixed";
    el.style.left = "-10000px";
    el.style.top = "0";
    el.style.width = "210mm";
    el.style.background = "#ffffff";
    document.body.appendChild(el);
    setHost(el);
    hostRef(el);
    return () => {
      hostRef(null);
      el.remove();
      setHost(null);
    };
    // hostRef is expected to be stable across renders (usePrint
    // returns a stable callback). Re-creating the host on every
    // render would defeat the point.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return host ? createPortal(children, host) : null;
}
