// Token map → `:root { … } .dark { … }` CSS string. Used by the
// "Copy CSS" export action and by tests that snapshot the output.

import type { ThemeStyleProps, ThemeStyles } from "../types.js";

/** Serialise a `ThemeStyles` pair into a CSS string suitable for
 * pasting into a stylesheet. The light block is always emitted; the
 * dark block is only emitted if it has at least one declaration. */
export function generateCssString(styles: ThemeStyles): string {
  const light = renderBlock(":root", styles.light);
  const dark = Object.keys(styles.dark).length > 0
    ? `\n\n${renderBlock(".dark", styles.dark)}`
    : "";
  return `${light}${dark}\n`;
}

function renderBlock(selector: string, props: ThemeStyleProps): string {
  const lines = Object.entries(props)
    .map(([key, value]) => `  --${key}: ${value};`)
    .join("\n");
  return `${selector} {\n${lines}\n}`;
}

/** Serialise a `ThemeStyles` pair as YAML. Inline implementation —
 * avoiding a `js-yaml` dependency for ~25 lines of code. */
export function generateYamlString(styles: ThemeStyles): string {
  return [
    "theme_styles:",
    "  light:",
    ...renderYamlBlock(styles.light),
    "  dark:",
    ...renderYamlBlock(styles.dark),
    "",
  ].join("\n");
}

function renderYamlBlock(props: ThemeStyleProps): string[] {
  return Object.entries(props).map(([key, value]) => `    ${key}: ${yamlScalar(value)}`);
}

/** Quote any value containing characters YAML treats as syntax so the
 * round-trip is lossless. Conservative: quote anything that isn't pure
 * `[A-Za-z0-9._%/-]`. */
function yamlScalar(value: string): string {
  if (/^[A-Za-z0-9._%/-]+$/.test(value)) return value;
  return `"${value.replace(/"/g, '\\"')}"`;
}
