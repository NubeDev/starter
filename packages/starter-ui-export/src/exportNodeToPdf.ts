import { dimensionsMm, type PageOptions } from "./types";

/**
 * Capture a DOM subtree as a PNG and embed it in a single-page PDF
 * matching {@link PageOptions} (size + orientation + margins).
 *
 * Requires the consumer to `pnpm add html2canvas jspdf` — both are
 * declared as **optional peer dependencies** so this package stays
 * tiny for consumers that only want {@link printNode}.
 *
 * Returns the PDF as a `Blob` so the caller chooses how to deliver
 * it: trigger a download, `POST` to the server, stash in IndexedDB,
 * preview in an `<iframe>`, etc.
 *
 * @throws if `html2canvas` or `jspdf` aren't installed in the
 *         consumer's `node_modules`.
 */
export async function exportNodeToPdf(
  node: HTMLElement,
  options: PageOptions,
): Promise<Blob> {
  const [html2canvas, jspdf] = await Promise.all([
    importOptional<{
      default: (
        el: HTMLElement,
        opts?: Record<string, unknown>,
      ) => Promise<HTMLCanvasElement>;
    }>("html2canvas"),
    importOptional<{
      jsPDF: new (opts: {
        orientation: "portrait" | "landscape";
        unit: "mm";
        format: [number, number];
      }) => {
        addImage: (
          data: string,
          format: "PNG",
          x: number,
          y: number,
          w: number,
          h: number,
        ) => void;
        output: (type: "blob") => Blob;
      };
    }>("jspdf"),
  ]);

  const [pageW, pageH] = dimensionsMm(options);
  const m = options.margins;
  const innerW = Math.max(1, pageW - m.left_mm - m.right_mm);
  const innerH = Math.max(1, pageH - m.top_mm - m.bottom_mm);

  const canvas = await html2canvas.default(node, {
    scale: 2,
    backgroundColor: "#ffffff",
  });

  // Letterbox the capture into the page's inner box so the aspect
  // ratio is preserved.
  const ratio = Math.min(innerW / canvas.width, innerH / canvas.height);
  const drawW = canvas.width * ratio;
  const drawH = canvas.height * ratio;
  const offX = m.left_mm + (innerW - drawW) / 2;
  const offY = m.top_mm + (innerH - drawH) / 2;

  const pdf = new jspdf.jsPDF({
    orientation: options.orientation,
    unit: "mm",
    format: [pageW, pageH],
  });
  pdf.addImage(canvas.toDataURL("image/png"), "PNG", offX, offY, drawW, drawH);

  return pdf.output("blob");
}

async function importOptional<T>(name: string): Promise<T> {
  try {
    // `@vite-ignore` keeps bundlers from trying to pre-resolve.
    return (await import(/* @vite-ignore */ name)) as T;
  } catch (cause) {
    throw new Error(
      `@nube/starter-ui-export: \`${name}\` is required for exportNodeToPdf(). ` +
        `Install it with: pnpm add ${name}`,
      { cause },
    );
  }
}
