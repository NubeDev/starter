import { useId, type ChangeEvent, type JSX } from "react";

import {
  DEFAULT_PAGE_OPTIONS,
  type Margins,
  type Orientation,
  type PageOptions,
  type PageSize,
} from "./types";

/**
 * Props for {@link PageOptionsForm}.
 */
export interface PageOptionsFormProps {
  /** Current page-options value. Pass {@link DEFAULT_PAGE_OPTIONS} for the defaults. */
  value?: PageOptions;
  /** Fired whenever the user changes any sub-field. */
  onChange: (next: PageOptions) => void;
  /**
   * Optional `id` prefix. Lets you mount more than one form on the
   * same page without label/`htmlFor` collisions.
   */
  idPrefix?: string;
  /** Extra class names appended to the root `<div>`. */
  className?: string;
}

const NAMED_SIZES: ReadonlyArray<{
  value: Exclude<PageSize["kind"], "custom">;
  label: string;
}> = [
  { value: "a4", label: "A4 (210 × 297 mm)" },
  { value: "a3", label: "A3 (297 × 420 mm)" },
  { value: "a5", label: "A5 (148 × 210 mm)" },
  { value: "letter", label: 'US Letter (8.5" × 11")' },
  { value: "legal", label: 'US Legal (8.5" × 14")' },
  { value: "tabloid", label: 'US Tabloid (11" × 17")' },
];

/**
 * Headless-ish form for editing a {@link PageOptions} value:
 * page size dropdown, portrait/landscape toggle, four margin inputs,
 * and a custom-size escape hatch.
 *
 * No styling library is imported — only semantic HTML and a single
 * `data-starter-export-form` attribute on the root so consumers can
 * pin Tailwind / shadcn / CSS-module classes through {@link className}.
 */
export function PageOptionsForm({
  value = DEFAULT_PAGE_OPTIONS,
  onChange,
  idPrefix,
  className,
}: PageOptionsFormProps): JSX.Element {
  const reactId = useId();
  const idp = idPrefix ?? reactId;

  const updateMargins = (patch: Partial<Margins>) =>
    onChange({ ...value, margins: { ...value.margins, ...patch } });

  const updateOrientation = (orientation: Orientation) =>
    onChange({ ...value, orientation });

  const updateSize = (size: PageSize) => onChange({ ...value, size });

  const onSizeKindChange = (e: ChangeEvent<HTMLSelectElement>) => {
    const kind = e.target.value as PageSize["kind"];
    if (kind === "custom") {
      updateSize({ kind: "custom", width_mm: 210, height_mm: 297 });
    } else {
      updateSize({ kind });
    }
  };

  const isCustom = value.size.kind === "custom";

  return (
    <div data-starter-export-form className={className}>
      <fieldset>
        <legend>Page size</legend>
        <label htmlFor={`${idp}-size`}>Preset</label>
        <select
          id={`${idp}-size`}
          value={value.size.kind}
          onChange={onSizeKindChange}
        >
          {NAMED_SIZES.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
          <option value="custom">Custom…</option>
        </select>

        {isCustom && value.size.kind === "custom" ? (
          <>
            <label htmlFor={`${idp}-w`}>Width (mm)</label>
            <input
              id={`${idp}-w`}
              type="number"
              min={1}
              step={1}
              value={value.size.width_mm}
              onChange={(e) =>
                updateSize({
                  kind: "custom",
                  width_mm: Number(e.target.value) || 0,
                  height_mm: (value.size as { height_mm: number }).height_mm,
                })
              }
            />
            <label htmlFor={`${idp}-h`}>Height (mm)</label>
            <input
              id={`${idp}-h`}
              type="number"
              min={1}
              step={1}
              value={value.size.height_mm}
              onChange={(e) =>
                updateSize({
                  kind: "custom",
                  width_mm: (value.size as { width_mm: number }).width_mm,
                  height_mm: Number(e.target.value) || 0,
                })
              }
            />
          </>
        ) : null}
      </fieldset>

      <fieldset>
        <legend>Orientation</legend>
        <label>
          <input
            type="radio"
            name={`${idp}-orientation`}
            value="portrait"
            checked={value.orientation === "portrait"}
            onChange={() => updateOrientation("portrait")}
          />
          Portrait
        </label>
        <label>
          <input
            type="radio"
            name={`${idp}-orientation`}
            value="landscape"
            checked={value.orientation === "landscape"}
            onChange={() => updateOrientation("landscape")}
          />
          Landscape
        </label>
      </fieldset>

      <fieldset>
        <legend>Margins (mm)</legend>
        {(
          [
            ["top_mm", "Top"],
            ["right_mm", "Right"],
            ["bottom_mm", "Bottom"],
            ["left_mm", "Left"],
          ] as const
        ).map(([key, label]) => (
          <label key={key} htmlFor={`${idp}-${key}`}>
            {label}
            <input
              id={`${idp}-${key}`}
              type="number"
              min={0}
              step={1}
              value={value.margins[key]}
              onChange={(e) =>
                updateMargins({ [key]: Number(e.target.value) || 0 } as Partial<Margins>)
              }
            />
          </label>
        ))}
      </fieldset>
    </div>
  );
}
