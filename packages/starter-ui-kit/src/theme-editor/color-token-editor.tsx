// Grouped token editor. Each token row renders a swatch, a text input
// (for typing raw oklch / hex / rgb), and a native colour picker for
// the common case. Foreground/background pairs are tagged with a WCAG
// contrast badge.
//
// All edits funnel through the store's `setToken`; a single
// `checkpoint()` is taken on each commit (input blur) so the undo
// ring records logical edits, not per-keystroke deltas.

import { useCallback } from "react";

import {
  getContrastRatio,
  getContrastTier,
  toHexForPicker,
  useThemeEditorStore,
} from "@nube/starter-ui-core/theme-editor";
import type {
  ThemeMode,
  ThemeStyleKey,
  ThemeStyleProps,
} from "@nube/starter-ui-core/theme-editor";

import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { cn } from "../lib/utils";

/** Visual grouping for the editor. Order is the render order. The
 * `pairWith` field, if set, drives the WCAG contrast badge: the row's
 * value is compared against the value of `pairWith` in the same mode. */
interface TokenGroup {
  title: string;
  rows: Array<{ key: ThemeStyleKey; label: string; pairWith?: ThemeStyleKey }>;
}

const GROUPS: readonly TokenGroup[] = [
  {
    title: "Brand",
    rows: [
      { key: "primary", label: "Primary", pairWith: "primary-foreground" },
      { key: "primary-foreground", label: "Primary foreground", pairWith: "primary" },
      { key: "accent", label: "Accent", pairWith: "accent-foreground" },
      { key: "accent-foreground", label: "Accent foreground", pairWith: "accent" },
      { key: "ring", label: "Focus ring" },
    ],
  },
  {
    title: "Surface",
    rows: [
      { key: "background", label: "Background", pairWith: "foreground" },
      { key: "foreground", label: "Foreground", pairWith: "background" },
      { key: "card", label: "Card", pairWith: "card-foreground" },
      { key: "card-foreground", label: "Card foreground", pairWith: "card" },
      { key: "popover", label: "Popover", pairWith: "popover-foreground" },
      { key: "popover-foreground", label: "Popover foreground", pairWith: "popover" },
      { key: "muted", label: "Muted", pairWith: "muted-foreground" },
      { key: "muted-foreground", label: "Muted foreground", pairWith: "muted" },
      { key: "secondary", label: "Secondary", pairWith: "secondary-foreground" },
      { key: "secondary-foreground", label: "Secondary foreground", pairWith: "secondary" },
      { key: "border", label: "Border" },
      { key: "input", label: "Input" },
    ],
  },
  {
    title: "Status",
    rows: [
      { key: "destructive", label: "Destructive", pairWith: "destructive-foreground" },
      { key: "destructive-foreground", label: "Destructive foreground", pairWith: "destructive" },
    ],
  },
  {
    title: "Sidebar",
    rows: [
      { key: "sidebar", label: "Sidebar", pairWith: "sidebar-foreground" },
      { key: "sidebar-foreground", label: "Sidebar foreground", pairWith: "sidebar" },
      { key: "sidebar-primary", label: "Sidebar primary", pairWith: "sidebar-primary-foreground" },
      { key: "sidebar-primary-foreground", label: "Sidebar primary foreground", pairWith: "sidebar-primary" },
      { key: "sidebar-accent", label: "Sidebar accent", pairWith: "sidebar-accent-foreground" },
      { key: "sidebar-accent-foreground", label: "Sidebar accent foreground", pairWith: "sidebar-accent" },
      { key: "sidebar-border", label: "Sidebar border" },
      { key: "sidebar-ring", label: "Sidebar focus ring" },
    ],
  },
  {
    title: "Charts",
    rows: [
      { key: "chart-1", label: "Chart 1" },
      { key: "chart-2", label: "Chart 2" },
      { key: "chart-3", label: "Chart 3" },
      { key: "chart-4", label: "Chart 4" },
      { key: "chart-5", label: "Chart 5" },
    ],
  },
  {
    title: "Shape",
    rows: [{ key: "radius", label: "Radius" }],
  },
  {
    title: "Typography",
    rows: [
      { key: "font-sans", label: "Sans family" },
      { key: "font-serif", label: "Serif family" },
      { key: "font-mono", label: "Mono family" },
      { key: "letter-spacing", label: "Letter spacing" },
    ],
  },
  {
    title: "Shadow",
    rows: [
      { key: "shadow-color", label: "Colour" },
      { key: "shadow-opacity", label: "Opacity" },
      { key: "shadow-blur", label: "Blur" },
      { key: "shadow-spread", label: "Spread" },
      { key: "shadow-offset-x", label: "Offset X" },
      { key: "shadow-offset-y", label: "Offset Y" },
    ],
  },
];

/** Tokens that should render a colour swatch + picker (vs. a plain text
 * input for fonts, sizes, opacities). Computed once. */
const COLOR_KEYS: ReadonlySet<ThemeStyleKey> = new Set(
  GROUPS.flatMap((g) =>
    g.rows
      .map((r) => r.key)
      .filter(
        (k) =>
          k !== "radius" &&
          k !== "font-sans" &&
          k !== "font-serif" &&
          k !== "font-mono" &&
          k !== "letter-spacing" &&
          k !== "shadow-opacity" &&
          k !== "shadow-blur" &&
          k !== "shadow-spread" &&
          k !== "shadow-offset-x" &&
          k !== "shadow-offset-y",
      ),
  ) as ThemeStyleKey[],
);

export interface ColorTokenEditorProps {
  /** Optional className for the outer wrapper. */
  className?: string;
}

export function ColorTokenEditor({ className }: ColorTokenEditorProps) {
  const mode = useThemeEditorStore((s) => s.mode);
  const styles = useThemeEditorStore((s) => s.styles);

  return (
    <div className={cn("flex flex-col gap-6", className)}>
      {GROUPS.map((group) => (
        <section key={group.title} aria-labelledby={`group-${group.title}`} className="flex flex-col gap-2">
          <h3 id={`group-${group.title}`} className="text-sm font-semibold text-foreground">
            {group.title}
          </h3>
          <div className="flex flex-col gap-2">
            {group.rows.map((row) => (
              <TokenRow
                key={row.key}
                tokenKey={row.key}
                label={row.label}
                pairWith={row.pairWith}
                mode={mode}
                map={styles[mode]}
              />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

interface TokenRowProps {
  tokenKey: ThemeStyleKey;
  label: string;
  pairWith?: ThemeStyleKey;
  mode: ThemeMode;
  map: ThemeStyleProps;
}

function TokenRow({ tokenKey, label, pairWith, mode, map }: TokenRowProps) {
  const setToken = useThemeEditorStore((s) => s.setToken);
  const checkpoint = useThemeEditorStore((s) => s.checkpoint);
  const value = map[tokenKey] ?? "";
  const isColor = COLOR_KEYS.has(tokenKey);

  const commit = useCallback(
    (next: string) => {
      checkpoint();
      setToken(mode, tokenKey, next);
    },
    [checkpoint, setToken, mode, tokenKey],
  );

  return (
    <div className="grid grid-cols-[1fr_auto] items-center gap-2">
      <div className="grid grid-cols-[auto_1fr_auto] items-center gap-2">
        {isColor ? (
          <input
            type="color"
            value={toHexForPicker(value)}
            onChange={(e) => commit(e.target.value)}
            aria-label={`${label} colour picker`}
            className="size-8 cursor-pointer rounded border border-border bg-transparent"
          />
        ) : (
          <span className="size-8" aria-hidden />
        )}
        <div className="flex flex-col gap-1">
          <Label htmlFor={`token-${tokenKey}`} className="text-xs text-muted-foreground">
            {label}
          </Label>
          <Input
            id={`token-${tokenKey}`}
            value={value}
            onChange={(e) => setToken(mode, tokenKey, e.target.value)}
            onBlur={() => checkpoint()}
            className="h-8 font-mono text-xs"
            spellCheck={false}
          />
        </div>
      </div>
      {pairWith ? <ContrastBadge a={value} b={map[pairWith] ?? ""} /> : <span />}
    </div>
  );
}

function ContrastBadge({ a, b }: { a: string; b: string }) {
  const ratio = getContrastRatio(a, b);
  const tier = getContrastTier(ratio);
  const colour =
    tier === "AAA"
      ? "bg-emerald-100 text-emerald-900 dark:bg-emerald-900/30 dark:text-emerald-200"
      : tier === "AA"
        ? "bg-amber-100 text-amber-900 dark:bg-amber-900/30 dark:text-amber-200"
        : "bg-rose-100 text-rose-900 dark:bg-rose-900/30 dark:text-rose-200";
  return (
    <span
      className={cn("rounded px-1.5 py-0.5 text-[10px] font-medium tabular-nums", colour)}
      title={ratio != null ? `Contrast ${ratio.toFixed(2)}:1` : "Invalid colour"}
    >
      {tier === "fail" ? "Fail" : tier} · {ratio != null ? ratio.toFixed(1) : "—"}
    </span>
  );
}
