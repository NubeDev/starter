// Generalized, props-driven sections for the ConfigDrawer. Each one
// takes the current value, a setter, a list of items (with
// host-localized labels), the default value (used to enable the
// "reset" affordance), and a bundle of aria strings.
//
// These deliberately don't import any consumer state hooks — ui-kit
// stays state-agnostic per R6. Hosts wire `useLayoutPreferences` (or
// their own facade) into the props.

import { type ReactNode, type SVGProps, type ReactElement } from "react";
import { RadioGroup as RadioGroupPrimitive } from "radix-ui";
import { CircleCheck } from "lucide-react";
import { cn } from "../../lib/utils.js";
import { RadioIconTile, RadioTile, SectionTitle } from "./shared.js";

const { Root: Radio, Item } = RadioGroupPrimitive;

/** Strings the host hands the section so every label / aria attribute
 * can be i18n-localized. Each section has a small i18n surface; the
 * host should derive these from its own `useTranslate`-style hook. */
export interface SectionI18n {
  /** Section heading (e.g. "Theme", "Palette"). */
  title: string;
  /** aria-label for the radio group. */
  groupAriaLabel: string;
  /** aria-label for the reset button. */
  resetAriaLabel: string;
}

/** A single option row, fully i18n-prepared by the host. */
export interface RadioOption<V extends string> {
  value: V;
  /** Localized human label. */
  label: string;
  /** Optional accessible label override (defaults to "Select <label>"
   * inside the primitive). */
  ariaLabel?: string;
}

// =========================================================================
// ThemeSection — light / dark / system tri-state. Items render via
// custom SVG icons supplied by the host (so we don't ship a specific
// IconThemeLight in ui-kit).
// =========================================================================

export interface ThemeIconItem<V extends string> extends RadioOption<V> {
  icon: (props: SVGProps<SVGSVGElement>) => ReactElement;
}

export interface ThemeSectionProps<V extends string = string> {
  value: V;
  onChange: (next: V) => void;
  items: ThemeIconItem<V>[];
  defaultValue: V;
  i18n: SectionI18n;
  className?: string;
}

export function ThemeSection<V extends string>({
  value,
  onChange,
  items,
  defaultValue,
  i18n,
  className,
}: ThemeSectionProps<V>) {
  return (
    <div className={className}>
      <SectionTitle
        title={i18n.title}
        showReset={value !== defaultValue}
        onReset={() => onChange(defaultValue)}
        resetAriaLabel={i18n.resetAriaLabel}
      />
      <Radio
        value={value}
        onValueChange={(v) => onChange(v as V)}
        className="grid w-full max-w-md grid-cols-3 gap-4"
        aria-label={i18n.groupAriaLabel}
      >
        {items.map((item) => (
          <RadioIconTile
            key={item.value}
            item={item}
            isTheme
            ariaLabel={item.ariaLabel}
          />
        ))}
      </Radio>
    </div>
  );
}

// =========================================================================
// PaletteSection — N swatches. Each item carries a CSS background
// (linear-gradient / solid colour) supplied by the host.
// =========================================================================

export interface PaletteItem<V extends string> extends RadioOption<V> {
  /** CSS `background` value (typically a `linear-gradient(...)`). */
  swatch: string;
}

export interface PaletteSectionProps<V extends string = string> {
  value: V;
  onChange: (next: V) => void;
  items: PaletteItem<V>[];
  defaultValue: V;
  i18n: SectionI18n;
  /** Optional grid column count; defaults to `3`. */
  columns?: number;
  className?: string;
}

export function PaletteSection<V extends string>({
  value,
  onChange,
  items,
  defaultValue,
  i18n,
  columns = 3,
  className,
}: PaletteSectionProps<V>) {
  return (
    <div className={className}>
      <SectionTitle
        title={i18n.title}
        showReset={value !== defaultValue}
        onReset={() => onChange(defaultValue)}
        resetAriaLabel={i18n.resetAriaLabel}
      />
      <Radio
        value={value}
        onValueChange={(v) => onChange(v as V)}
        className={cn("grid w-full max-w-md gap-4", gridColsClass(columns))}
        aria-label={i18n.groupAriaLabel}
      >
        {items.map((item) => (
          <Item
            key={item.value}
            value={item.value}
            className="group outline-none"
            aria-label={item.ariaLabel ?? `Select ${item.label.toLowerCase()}`}
          >
            <div
              className={cn(
                "relative rounded-[6px] ring-[1px] ring-[color:var(--color-border)] p-3",
                "group-data-[state=checked]:shadow-2xl group-data-[state=checked]:ring-[color:var(--color-leaf)]",
              )}
            >
              <CircleCheck
                className={cn(
                  "size-6 fill-[color:var(--color-leaf)] stroke-white",
                  "group-data-[state=unchecked]:hidden",
                  "absolute top-0 right-0 translate-x-1/2 -translate-y-1/2",
                )}
              />
              <div className="h-10 w-full rounded" style={{ background: item.swatch }} />
            </div>
            <div className="mt-1 text-xs">{item.label}</div>
          </Item>
        ))}
      </Radio>
    </div>
  );
}

// =========================================================================
// FontSizeSection — Aa swatches at three pixel sizes.
// =========================================================================

export interface FontSizeItem<V extends string> extends RadioOption<V> {
  /** CSS font-size to render the "Aa" preview at, e.g. "14px". */
  px: string;
}

export interface FontSizeSectionProps<V extends string = string> {
  value: V;
  onChange: (next: V) => void;
  items: FontSizeItem<V>[];
  defaultValue: V;
  i18n: SectionI18n;
  className?: string;
}

export function FontSizeSection<V extends string>({
  value,
  onChange,
  items,
  defaultValue,
  i18n,
  className,
}: FontSizeSectionProps<V>) {
  return (
    <div className={className}>
      <SectionTitle
        title={i18n.title}
        showReset={value !== defaultValue}
        onReset={() => onChange(defaultValue)}
        resetAriaLabel={i18n.resetAriaLabel}
      />
      <Radio
        value={value}
        onValueChange={(v) => onChange(v as V)}
        className="grid w-full max-w-md grid-cols-3 gap-2"
        aria-label={i18n.groupAriaLabel}
      >
        {items.map((item) => (
          <RadioTile key={item.value} value={item.value} label={item.label} ariaLabel={item.ariaLabel}>
            <div className="font-medium text-[color:var(--color-text)]" style={{ fontSize: item.px }}>
              Aa
            </div>
            <div className="text-[10px] text-[color:var(--color-subtle)]">{item.label}</div>
          </RadioTile>
        ))}
      </Radio>
    </div>
  );
}

// =========================================================================
// TileChoiceSection — generic "N tiles, each with a localized label
// and a tiny secondary line". Powers Density + Motion in the ui-5
// drawer. Hosts that need a richer rendering can pass `renderItem`.
// =========================================================================

export interface TileChoiceItem<V extends string> extends RadioOption<V> {
  /** Optional secondary line beneath the label inside the tile. */
  hint?: string;
}

export interface TileChoiceSectionProps<V extends string = string> {
  value: V;
  onChange: (next: V) => void;
  items: TileChoiceItem<V>[];
  defaultValue: V;
  i18n: SectionI18n;
  /** Grid column count. Defaults to `items.length`, capped at 4. */
  columns?: number;
  /** Tailwind gap class. Defaults to `gap-2`. */
  gapClassName?: string;
  /** Custom renderer for the tile body. Falls back to a centered
   * `<div>{label}</div>`. */
  renderItem?: (item: TileChoiceItem<V>) => ReactNode;
  className?: string;
}

export function TileChoiceSection<V extends string>({
  value,
  onChange,
  items,
  defaultValue,
  i18n,
  columns,
  gapClassName = "gap-2",
  renderItem,
  className,
}: TileChoiceSectionProps<V>) {
  const cols = columns ?? Math.min(items.length, 4);
  return (
    <div className={className}>
      <SectionTitle
        title={i18n.title}
        showReset={value !== defaultValue}
        onReset={() => onChange(defaultValue)}
        resetAriaLabel={i18n.resetAriaLabel}
      />
      <Radio
        value={value}
        onValueChange={(v) => onChange(v as V)}
        className={cn("grid w-full max-w-md", gridColsClass(cols), gapClassName)}
        aria-label={i18n.groupAriaLabel}
      >
        {items.map((item) => (
          <RadioTile key={item.value} value={item.value} label={item.label} ariaLabel={item.ariaLabel}>
            {renderItem ? (
              renderItem(item)
            ) : (
              <div className="text-sm font-medium text-[color:var(--color-text)]">{item.label}</div>
            )}
          </RadioTile>
        ))}
      </Radio>
    </div>
  );
}

// Tailwind v4 needs concrete class strings to scan; pick from a small
// switch rather than building `grid-cols-${n}` dynamically.
function gridColsClass(n: number): string {
  switch (n) {
    case 1: return "grid-cols-1";
    case 2: return "grid-cols-2";
    case 3: return "grid-cols-3";
    case 4: return "grid-cols-4";
    default: return "grid-cols-3";
  }
}
