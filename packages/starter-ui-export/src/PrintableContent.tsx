import { useEffect, useState, type JSX, type ReactNode } from "react";
import { createPortal } from "react-dom";

/**
 * Props for {@link PrintableContent}.
 */
export interface PrintableContentProps {
  /** React tree to render into the hidden print host. */
  children: ReactNode;
  /**
   * Fired *after* `children` have committed into the hidden host —
   * passes the host element. Fired again with `null` on unmount.
   * Use this to safely call {@link printNode} or {@link exportNodeToPdf}
   * once the printable subtree is in the DOM.
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
 *
 * `hostRef` only fires once `children` have actually committed to the
 * host; consumers can therefore call `printNode(host)` synchronously
 * inside the callback without racing the portal's first paint.
 */
export function PrintableContent({
  children,
  hostRef,
}: PrintableContentProps): JSX.Element | null {
  const [host, setHost] = useState<HTMLDivElement | null>(null);

  useEffect(() => {
    // Offscreen positioning lives in a screen-only stylesheet so the
    // print-only stylesheet from `printNode` (which positions the
    // host at 0,0) can win without inline-style specificity getting
    // in the way.
    ensureHostStyle();
    const el = document.createElement("div");
    el.setAttribute("data-starter-export-host", "");
    document.body.appendChild(el);
    setHost(el);
    return () => {
      hostRef(null);
      el.remove();
      setHost(null);
    };
    // hostRef intentionally omitted — see the second effect for the
    // post-commit notification.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Fires *after* the portal commits `children` into `host`, so the
  // consumer's `hostRef` callback sees a populated subtree.
  useEffect(() => {
    if (host) hostRef(host);
    // hostRef is expected to be stable.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [host, children]);

  return host ? createPortal(children, host) : null;
}

const HOST_STYLE_ID = "starter-export-host-style";

function ensureHostStyle(): void {
  if (document.getElementById(HOST_STYLE_ID)) return;
  const style = document.createElement("style");
  style.id = HOST_STYLE_ID;
  // Screen rules push the host offscreen so it doesn't disturb the
  // visible page. Print rules are intentionally absent here — the
  // print-only stylesheet from `printNode` owns positioning during
  // print so it can win without specificity battles.
  style.textContent = `
    @media screen {
      [data-starter-export-host] {
        position: fixed;
        left: -10000px;
        top: 0;
        width: 210mm;
        background: #ffffff;
        print-color-adjust: exact;
        -webkit-print-color-adjust: exact;
      }
    }
    @media print {
      [data-starter-export-host] {
        background: #ffffff;
        print-color-adjust: exact;
        -webkit-print-color-adjust: exact;
      }
    }
  `;
  document.head.appendChild(style);
}
