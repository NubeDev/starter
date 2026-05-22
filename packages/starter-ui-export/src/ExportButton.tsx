import { useState, type JSX, type ReactNode } from "react";

import { exportNodeToPdf } from "./exportNodeToPdf";
import { printNode } from "./printNode";
import { DEFAULT_PAGE_OPTIONS, type PageOptions } from "./types";

/**
 * Props for {@link ExportButton}.
 */
export interface ExportButtonProps {
  /** Function that returns the DOM node to export. Called lazily on click. */
  target: () => HTMLElement | null;
  /** Page options. Defaults to {@link DEFAULT_PAGE_OPTIONS}. */
  options?: PageOptions;
  /**
   * Strategy for producing the PDF.
   *
   * - `"print"`: open the native print dialog. Zero deps. User picks
   *   "Save as PDF" if they want a file.
   * - `"download"`: silently render to PDF via `html2canvas` + `jspdf`
   *   and trigger a download with `filename`. Requires those optional
   *   peer deps.
   */
  mode?: "print" | "download";
  /** Filename (no extension) used by `mode: "download"`. */
  filename?: string;
  /** Button label. Defaults to "Export PDF". */
  children?: ReactNode;
  /** Class names for the underlying `<button>`. */
  className?: string;
}

/**
 * One-click "export this DOM node to PDF" button.
 *
 * Stays zero-I/O (SCOPE.md R6): all work happens client-side. If the
 * consumer wants to ship the bytes to a server, they can do it
 * themselves — or use `mode: "print"` and let the user choose
 * "Save as PDF" in the browser.
 */
export function ExportButton({
  target,
  options = DEFAULT_PAGE_OPTIONS,
  mode = "print",
  filename = "export",
  children = "Export PDF",
  className,
}: ExportButtonProps): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const onClick = async () => {
    setError(null);
    const node = target();
    if (!node) {
      setError("nothing to export");
      return;
    }
    if (mode === "print") {
      printNode(node, options);
      return;
    }
    setBusy(true);
    try {
      const blob = await exportNodeToPdf(node, options);
      triggerDownload(blob, `${filename}.pdf`);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={busy}
      aria-busy={busy || undefined}
      className={className}
      data-starter-export-button
    >
      {busy ? "Working…" : children}
      {error ? (
        <span role="alert" data-starter-export-error>
          {" "}
          — {error}
        </span>
      ) : null}
    </button>
  );
}

function triggerDownload(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  // Give the browser a tick to start the download before revoking.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
