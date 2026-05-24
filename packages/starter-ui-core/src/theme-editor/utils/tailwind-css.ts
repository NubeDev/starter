// Token map → Tailwind v4 `@theme` block.
//
// `generateCssString` (in `generate-css.ts`) emits the classic
// `:root { … } .dark { … }` form that any stylesheet can drop in.
// Tailwind v4 consumers prefer the CSS-first `@theme` directive so the
// JIT picks the tokens up as utility values (e.g.
// `bg-(--color-primary)` becomes a class generator).
//
// This generator is **opt-in**. ui-core stays framework-agnostic; only
// consumers that already build on Tailwind v4 should call it.

import type { ThemeStyleProps, ThemeStyles } from "../types.js";
import { NON_COLOR_KEYS } from "../defaults.js";

/** Emit a Tailwind v4 `@theme` block plus a `.dark @theme` override.
 *
 * Light tokens become `--color-<key>` / `--<key>` defaults. Dark
 * tokens land inside `.dark { @theme { … } }` so the cascade flips
 * them when the `dark` class is on `<html>`. The non-colour tokens
 * (radius, fonts, shadow) emit verbatim — no `--color-` prefix. */
export function generateTailwindThemeCss(styles: ThemeStyles): string {
  const lightBlock = renderThemeBlock(styles.light);
  const hasDark = Object.keys(styles.dark).length > 0;
  const darkBlock = hasDark
    ? `\n\n.dark {\n  @theme inline {\n${renderEntries(styles.dark, "    ")}\n  }\n}`
    : "";
  return `@theme inline {\n${lightBlock}\n}${darkBlock}\n`;
}

function renderThemeBlock(props: ThemeStyleProps): string {
  return renderEntries(props, "  ");
}

function renderEntries(props: ThemeStyleProps, indent: string): string {
  return Object.entries(props)
    .map(([key, value]) => `${indent}${prefixedKey(key)}: ${value};`)
    .join("\n");
}

/** Colour tokens become `--color-<key>` so Tailwind v4 generates the
 * `bg-<key>` / `text-<key>` utilities automatically. Non-colour tokens
 * keep their raw name. */
function prefixedKey(key: string): string {
  return NON_COLOR_KEYS.has(key) ? `--${key}` : `--color-${key}`;
}
