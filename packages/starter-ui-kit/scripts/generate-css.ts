/**
 * Generate `src/styles/globals.css` from `@nube/starter-theme-tokens`.
 *
 * The hand-edited CSS is gone: tokens live in
 * `packages/starter-theme-tokens/src/palette.ts` (and friends) and
 * this script renders the kit's stylesheet from them. The output is
 * checked in to keep `pnpm dev` zero-config — re-run this script
 * whenever tokens change.
 *
 * Acceptance bar: when run against today's token values, the emitted
 * CSS is **byte-identical** to `scripts/__fixtures__/globals.expected.css`.
 * That regression fixture is the contract that lets the next stages
 * (RN packages) consume the same token source without drifting.
 *
 * Usage:
 *   pnpm --filter @nube/starter-ui-kit generate:css     # write the file
 *   pnpm --filter @nube/starter-ui-kit verify:css       # check no drift
 *
 * Run with `tsx` (no compile step). The script has no React/DOM imports.
 */

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  CSS_EMISSION_ORDER_DARK,
  CSS_EMISSION_ORDER_LIGHT,
  type ThemeStyleKey,
  type ThemeTokenMap,
  platformDarkPalette,
  platformLightPalette,
} from "@nube/starter-theme-tokens";
import { RADIUS_MULTIPLIERS } from "@nube/starter-theme-tokens/radius";
import { FONT_SANS_STACK } from "@nube/starter-theme-tokens/type";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_PATH = resolve(HERE, "..", "src", "styles", "globals.css");
const FIXTURE_PATH = resolve(HERE, "__fixtures__", "globals.expected.css");

function renderTokens(palette: ThemeTokenMap, order: readonly ThemeStyleKey[]): string {
  return order
    .map((key) => {
      const value = palette[key];
      if (value === undefined) {
        throw new Error(`Missing token "${key}" in palette`);
      }
      return `    --${key}: ${value};`;
    })
    .join("\n");
}

function renderRadiusScale(): string {
  return Object.entries(RADIUS_MULTIPLIERS)
    .map(([name, mult]) => {
      const rhs = mult === 1 ? "var(--radius)" : `calc(var(--radius) * ${mult})`;
      return `    --radius-${name}: ${rhs};`;
    })
    .join("\n");
}

export function generateCss(): string {
  const lightBlock = renderTokens(platformLightPalette, CSS_EMISSION_ORDER_LIGHT);
  const darkBlock = renderTokens(platformDarkPalette, CSS_EMISSION_ORDER_DARK);
  const radiusBlock = renderRadiusScale();

  return `@import "tailwindcss";
@import "tw-animate-css";
@import "@fontsource-variable/inter";

/* Ensure Tailwind v4 scans this package's own components when a host
 * imports this stylesheet from node_modules (pnpm). v4 skips node_modules
 * by default, so without this \`@source\` the kit's shadcn primitives would
 * render unstyled (no border-radius, no spacing, no colours). */
@source "../..";

@custom-variant dark (&:is(.dark *));

@theme inline {
    --font-sans: ${FONT_SANS_STACK};
    --font-heading: var(--font-sans);
    --color-sidebar-ring: var(--sidebar-ring);
    --color-sidebar-border: var(--sidebar-border);
    --color-sidebar-accent-foreground: var(--sidebar-accent-foreground);
    --color-sidebar-accent: var(--sidebar-accent);
    --color-sidebar-primary-foreground: var(--sidebar-primary-foreground);
    --color-sidebar-primary: var(--sidebar-primary);
    --color-sidebar-foreground: var(--sidebar-foreground);
    --color-sidebar: var(--sidebar);
    --color-chart-5: var(--chart-5);
    --color-chart-4: var(--chart-4);
    --color-chart-3: var(--chart-3);
    --color-chart-2: var(--chart-2);
    --color-chart-1: var(--chart-1);
    --color-ring: var(--ring);
    --color-input: var(--input);
    --color-border: var(--border);
    --color-destructive: var(--destructive);
    --color-accent-foreground: var(--accent-foreground);
    --color-accent: var(--accent);
    --color-muted-foreground: var(--muted-foreground);
    --color-muted: var(--muted);
    --color-secondary-foreground: var(--secondary-foreground);
    --color-secondary: var(--secondary);
    --color-primary-foreground: var(--primary-foreground);
    --color-primary: var(--primary);
    --color-popover-foreground: var(--popover-foreground);
    --color-popover: var(--popover);
    --color-card-foreground: var(--card-foreground);
    --color-card: var(--card);
    --color-foreground: var(--foreground);
    --color-background: var(--background);
${radiusBlock}
}

:root {
${lightBlock}
}

.dark {
${darkBlock}
}

@layer base {
  * {
    @apply border-border outline-ring/50;
  }
  html {
    @apply font-sans;
  }
  body {
    @apply bg-background text-foreground;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
}
`;
}

function main(): void {
  const mode = process.argv[2] ?? "write";
  const css = generateCss();

  if (mode === "verify") {
    const onDisk = readFileSync(OUT_PATH, "utf8");
    if (onDisk !== css) {
      console.error(
        `globals.css is out of sync with @nube/starter-theme-tokens.\n` +
          `Run: pnpm --filter @nube/starter-ui-kit generate:css`,
      );
      process.exit(1);
    }
    const fixture = readFileSync(FIXTURE_PATH, "utf8");
    if (fixture !== css) {
      console.error(
        `Regression fixture drift: scripts/__fixtures__/globals.expected.css ` +
          `does not match the freshly-generated output. If this is an intended ` +
          `token change, update the fixture and call out the visual diff in PR review.`,
      );
      process.exit(1);
    }
    console.log("globals.css is in sync with tokens and fixture.");
    return;
  }

  writeFileSync(OUT_PATH, css);
  console.log(`Wrote ${OUT_PATH}`);
}

// Node ESM entry-point guard.
if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
