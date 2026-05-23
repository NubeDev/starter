# @nube/starter-ui-export

Browser-side PDF / print UI for the starter ecosystem. Zero I/O
(SCOPE.md R6): every export runs in the user's browser. Pair it with
the `starter-export` Rust crate when you want the bytes to come from
the server instead.

## Install

```sh
pnpm add @nube/starter-ui-export
```

Optional — only if you use `mode="download"` / `exportNodeToPdf`:

```sh
pnpm add html2canvas jspdf
```

## Why a frontend path at all?

Pure-Rust PDF backends (printpdf, genpdf, …) are great for tabular
reports but they are not browser engines: CSS, web fonts, flexbox,
charts and arbitrary HTML are not rendered. When the consumer needs a
PDF that **looks like the page the user is staring at**, generating
it client-side is dramatically simpler and produces a better result.

This package gives you that path without locking you into a specific
PDF library: pick `printNode` (zero deps, native browser print) or
`exportNodeToPdf` (silent, optional `html2canvas` + `jspdf`).

## Recommended: `usePrint` + `<PrintableContent>`

For most apps you want the print view rendered *off-screen* (so it
doesn't pollute the visible page) and triggered by your own styled
button. The `usePrint` hook + `PrintableContent` portal handle both:

```tsx
import {
  PrintableContent,
  usePrint,
  DEFAULT_PAGE_OPTIONS,
} from "@nube/starter-ui-export";

export function Report() {
  const { hostRef, print, printing } = usePrint(DEFAULT_PAGE_OPTIONS);

  return (
    <>
      <button onClick={print} disabled={printing}>
        {printing ? "Preparing…" : "Print / Save as PDF"}
      </button>
      <PrintableContent hostRef={hostRef}>
        <MyReport />
      </PrintableContent>
    </>
  );
}
```

`PrintableContent` mounts its children into a hidden 210 mm-wide
container on `document.body`. `print()` waits for `document.fonts.ready`
and `img.decode()` to settle, then opens the native print dialog.

## Lower-level building blocks

```tsx
import {
  PageOptionsForm,
  ExportButton,
  printNode,
  exportNodeToPdf,
  DEFAULT_PAGE_OPTIONS,
  type PageOptions,
} from "@nube/starter-ui-export";

export function Report() {
  const [opts, setOpts] = useState<PageOptions>(DEFAULT_PAGE_OPTIONS);
  const ref = useRef<HTMLDivElement>(null);

  return (
    <>
      <PageOptionsForm value={opts} onChange={setOpts} />
      <ExportButton target={() => ref.current} options={opts} mode="print">
        Print / Save as PDF
      </ExportButton>
      <ExportButton
        target={() => ref.current}
        options={opts}
        mode="download"
        filename="quarterly-report"
      >
        Download PDF
      </ExportButton>
      <div ref={ref}>{/* the page you want to export */}</div>
    </>
  );
}
```

### `PageOptionsForm`

Headless form for editing a `PageOptions` value: A4 / A3 / A5 /
Letter / Legal / Tabloid / Custom, portrait/landscape, four margins.
No styling library is imported — semantic HTML only, attach Tailwind
/ shadcn / CSS modules through `className` and target the root
`[data-starter-export-form]` selector.

### `printNode(node, options)`

Opens the browser's native print dialog with an injected `@page` rule
matching the supplied size / orientation / margins. The user picks
"Save as PDF" if they want a file. Zero dependencies.

Returns a `Promise<void>` that resolves once fonts and inline images
are ready and the print dialog has been requested.

### `exportNodeToPdf(node, options)`

Captures the subtree with `html2canvas` and embeds the PNG in a
**single-page** PDF via `jspdf`, letterboxed into the configured page
size. Returns a `Blob` so the caller decides what to do with it:
download, `POST` to your server, preview in an `<iframe>`. Both
libraries are declared as **optional peer deps** so consumers who only
want `printNode` don't pay for them.

> **Note:** this path is single-page only — tall content is scaled
> down to fit. For multi-page documents use `printNode` (the browser
> handles pagination natively).

### `ExportButton`

Convenience wrapper that drives either of the two strategies above.
`mode="print"` for native, `mode="download"` for silent.

## Round-tripping with `starter-export` (Rust)

The TypeScript `PageOptions` interface is the literal shape the Rust
`starter_export::PageOptions` struct serialises to, so a saved report
preset works on both sides:

```ts
fetch("/v1/export", {
  method: "POST",
  body: JSON.stringify({
    format: "pdf",
    page: opts,
    payload: { title: "Q1", sections: [...] },
  }),
});
```

Pick the right backend per request: send tabular reports to the Rust
endpoint, render rich pages on the frontend.
