// `:root { … }` / `.dark { … }` CSS → token map parser. Pure string
// work; no DOM, no eval. Used by `ImportCssDialog` and by consumers
// who want to round-trip a theme through git.
//
// Adapted from tweakcn (https://github.com/jnsahaj/tweakcn).
// Original work Copyright (c) 2024 Sahaj Jain. Apache License 2.0.
// Modifications Copyright (c) starter contributors.

import type { ThemeStyleProps, ThemeStyles } from "../types.js";

/** Extract token maps from a CSS string containing zero or more
 * `:root { … }` and `.dark { … }` blocks. Tokens outside those blocks
 * are ignored. Unknown `--*` properties are passed through — the
 * editor will silently drop any key it doesn't render. */
export function parseCssInput(css: string): Partial<ThemeStyles> {
  const out: Partial<ThemeStyles> = {};
  const light = extractBlock(css, /:root\s*\{([^}]*)\}/);
  const dark = extractBlock(css, /\.dark\s*\{([^}]*)\}/);
  if (light) out.light = light;
  if (dark) out.dark = dark;
  return out;
}

function extractBlock(css: string, pattern: RegExp): ThemeStyleProps | null {
  const match = pattern.exec(css);
  if (!match) return null;
  const body = match[1] ?? "";
  const result: ThemeStyleProps = {};
  for (const decl of body.split(";")) {
    const [rawKey, ...rest] = decl.split(":");
    if (!rawKey || rest.length === 0) continue;
    const key = rawKey.trim();
    if (!key.startsWith("--")) continue;
    const value = rest.join(":").trim();
    if (!value) continue;
    // Strip the leading `--` so the in-memory shape matches `ThemeStyleKey`.
    (result as Record<string, string>)[key.slice(2)] = value;
  }
  return result;
}
