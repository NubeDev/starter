import { dimensionsMm, type PageOptions } from "./types";

/**
 * Open the browser's native print dialog with the given page options
 * applied through an injected `@page` rule scoped to the target node.
 *
 * Zero external dependencies. The user can choose "Save as PDF" in
 * the print dialog to get a real PDF — modern Chrome/Edge/Safari/
 * Firefox all support this.
 *
 * Awaits font loading and inline `<img>` decoding before opening the
 * dialog so the first preview isn't missing glyphs or image data.
 *
 * Trade-off: the user sees the OS print dialog. If you want a
 * silent, programmatic PDF without that dialog, use
 * {@link exportNodeToPdf} instead (it requires `html2canvas` + `jspdf`).
 *
 * @param node Element whose subtree should be printed. Everything
 *             else on the page is hidden via a print-only stylesheet.
 * @param options Page size / orientation / margins.
 */
export async function printNode(
  node: HTMLElement,
  options: PageOptions,
): Promise<void> {
  const [w, h] = dimensionsMm(options);
  const m = options.margins;

  const styleId = `starter-export-print-${Math.random().toString(36).slice(2)}`;
  const marker = "starter-export-print-target";
  node.setAttribute(marker, "");

  const style = document.createElement("style");
  style.id = styleId;
  style.media = "print";
  style.textContent = `
    @page { size: ${w}mm ${h}mm; margin: ${m.top_mm}mm ${m.right_mm}mm ${m.bottom_mm}mm ${m.left_mm}mm; }
    body * { visibility: hidden !important; }
    [${marker}], [${marker}] * { visibility: visible !important; }
    [${marker}] { position: absolute; left: 0; top: 0; width: 100%; }
  `;
  document.head.appendChild(style);

  const cleanup = () => {
    node.removeAttribute(marker);
    style.remove();
    window.removeEventListener("afterprint", cleanup);
  };
  window.addEventListener("afterprint", cleanup);

  try {
    await waitForReady(node);
    window.print();
  } catch (err) {
    cleanup();
    throw err;
  }
}

async function waitForReady(node: HTMLElement): Promise<void> {
  // Custom fonts are async-loaded; without this the first preview can
  // render with a fallback face and visibly re-flow on the second.
  const fontsReady =
    typeof document !== "undefined" && "fonts" in document
      ? (document as Document & { fonts: { ready: Promise<unknown> } }).fonts
          .ready
      : Promise.resolve();

  // `img.decode()` resolves once the bitmap is decoded and ready to
  // paint; `.catch(() => {})` so a single broken image can't block the
  // whole print.
  const imgs = Array.from(node.querySelectorAll("img"));
  const decoded = imgs.map((img) =>
    img.decode ? img.decode().catch(() => {}) : Promise.resolve(),
  );

  await Promise.all([fontsReady, ...decoded]);
}
