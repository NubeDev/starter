/**
 * Page-options vocabulary, mirrored from the `starter-export` Rust
 * crate so a `PageOptions` value round-trips losslessly across the
 * wire. Keep the union literals in sync with `page.rs`.
 */

export type Orientation = "portrait" | "landscape";

export type NamedPageSize =
  | { kind: "a4" }
  | { kind: "a3" }
  | { kind: "a5" }
  | { kind: "letter" }
  | { kind: "legal" }
  | { kind: "tabloid" };

export type CustomPageSize = {
  kind: "custom";
  width_mm: number;
  height_mm: number;
};

export type PageSize = NamedPageSize | CustomPageSize;

export interface Margins {
  top_mm: number;
  right_mm: number;
  bottom_mm: number;
  left_mm: number;
}

export interface PageOptions {
  size: PageSize;
  orientation: Orientation;
  margins: Margins;
}

export type ExportFormat = "pdf" | "html" | "csv" | "json" | "markdown";

/** Default 15 mm margins on every side. */
export const DEFAULT_MARGINS: Margins = {
  top_mm: 15,
  right_mm: 15,
  bottom_mm: 15,
  left_mm: 15,
};

/** Default `A4` portrait. */
export const DEFAULT_PAGE_OPTIONS: PageOptions = {
  size: { kind: "a4" },
  orientation: "portrait",
  margins: DEFAULT_MARGINS,
};

/** Width × height in mm, **after** applying orientation. */
export function dimensionsMm(opts: PageOptions): [number, number] {
  const base = baseDimensionsMm(opts.size);
  return opts.orientation === "landscape" ? [base[1], base[0]] : base;
}

function baseDimensionsMm(size: PageSize): [number, number] {
  switch (size.kind) {
    case "a4":
      return [210, 297];
    case "a3":
      return [297, 420];
    case "a5":
      return [148, 210];
    case "letter":
      return [215.9, 279.4];
    case "legal":
      return [215.9, 355.6];
    case "tabloid":
      return [279.4, 431.8];
    case "custom":
      return [size.width_mm, size.height_mm];
  }
}
