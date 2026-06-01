import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import cssInjectedByJsPlugin from "vite-plugin-css-injected-by-js";
import type { AcceptedPlugin } from "postcss";

// Build config — same recipe as `com.rubix.example`. The rubix host
// expects an SDK-shape factory (`{ singletons, init(handle) }`),
// React/ReactDOM externalised (importmap publishes the host's
// instances on `globalThis.__rubixReact*`), and a single ESM bundle
// emitted at `ui/remoteEntry.js`.

const EXTENSION_ID = "com.nubeio.rubixos";
const SCOPE_SELECTOR = `[data-ext-id="${EXTENSION_ID}"]`;

/**
 * `scope-extension-css` — PostCSS plugin that prefixes every selector
 * in the extension's bundled CSS with `[data-ext-id="…"]` so the
 * extension's utility rules ONLY apply inside its own DOM subtree.
 *
 * Without scoping, the extension's bundle ships `.grid-cols-1`,
 * `.flex`, `.bg-card`, etc. — identical class names to the host's
 * Tailwind bundle. Both load on the same page, both target the same
 * DOM nodes by class, and source-order tiebreaks pick a different
 * winner depending on which bundle loaded last. That coupling breaks
 * either the host's responsive variants (e.g. `lg:grid-cols-[260px_1fr_320px]`
 * on the flow editor) or the extension's own responsive variants
 * (e.g. `lg:grid-cols-[minmax(260px,320px)_1fr_minmax(220px,260px)]`
 * on the energy dashboard) — there's no source order that keeps both
 * authoritative.
 *
 * Scoping at the selector level resolves it permanently:
 *
 *   .grid-cols-1 { … }
 *
 * becomes
 *
 *   [data-ext-id="com.nubeio.rubixos"] .grid-cols-1,
 *   [data-ext-id="com.nubeio.rubixos"].grid-cols-1 { … }
 *
 * — the rule only matches a `.grid-cols-1` element that is, or lives
 * inside, an element with the extension's data attribute. The host's
 * `.grid-cols-1` rule is unchanged and authoritative everywhere else.
 *
 * Universal selectors (`*`, `:root`, `html`, `body`, keyframe steps,
 * percentage stops, and @-rule param strings) must be left alone, so
 * we only rewrite real selectors and skip the rest by inspection.
 */
function scopeExtensionCssPlugin(scope: string): AcceptedPlugin {
  // Selectors that target the page root / globals — DO NOT scope these.
  // Anything starting with `*`, `:root`, `html`, `body`, `:host`, or a
  // keyframe step (`from`, `to`, percentages) is left untouched.
  const SKIP_RE = /^(\*|:root\b|:host\b|html\b|body\b|from\b|to\b|\d+%|@)/;

  // Split a selector list on top-level commas (commas outside parens
  // and brackets), so each branch can be prefixed independently.
  const splitTopLevel = (list: string): string[] => {
    const out: string[] = [];
    let depth = 0;
    let buf = "";
    for (let i = 0; i < list.length; i++) {
      const c = list[i];
      if (c === "(" || c === "[") depth++;
      else if (c === ")" || c === "]") depth--;
      else if (c === "," && depth === 0) {
        out.push(buf.trim());
        buf = "";
        continue;
      }
      buf += c;
    }
    if (buf.trim()) out.push(buf.trim());
    return out;
  };

  const scopeOne = (sel: string): string => {
    const trimmed = sel.trim();
    if (!trimmed) return trimmed;
    if (SKIP_RE.test(trimmed)) return trimmed;
    // Two clauses: `scope sel` (descendant) AND `scope.sel` / `scope[…]`
    // (same element) so the prefix matches the root element itself if
    // it carries one of the utility classes (e.g. the extension root
    // div with `flex` on it).
    if (
      trimmed.startsWith(".") ||
      trimmed.startsWith("[") ||
      trimmed.startsWith(":") ||
      trimmed.startsWith("#")
    ) {
      return `${scope} ${trimmed}, ${scope}${trimmed}`;
    }
    return `${scope} ${trimmed}`;
  };

  return {
    postcssPlugin: "scope-extension-css",
    Rule(rule) {
      // Skip rules already inside @keyframes (the parent is an
      // at-rule whose name matches /keyframes/i). PostCSS walks
      // keyframe steps as Rules too and they must not be prefixed.
      let p = rule.parent;
      while (p) {
        if (p.type === "atrule") {
          const name = (p as { name?: string }).name || "";
          if (/keyframes$/i.test(name) || name === "property") return;
        }
        p = p.parent;
      }
      const next = splitTopLevel(rule.selector).map(scopeOne).join(", ");
      if (next !== rule.selector) rule.selector = next;
    },
  };
}

export default defineConfig({
  plugins: [tailwindcss(), cssInjectedByJsPlugin()],
  define: {
    // Surfaces in the browser console + on `window.__rubixosBuild`
    // so you can verify the page is running the freshly-built
    // remoteEntry.js (and not a cached older one) before chasing
    // CSS / specificity ghosts.
    __BUILD_STAMP__: JSON.stringify(new Date().toISOString()),
  },
  css: {
    postcss: {
      plugins: [scopeExtensionCssPlugin(SCOPE_SELECTOR)],
    },
  },
  build: {
    target: "esnext",
    outDir: "../ui",
    emptyOutDir: true,
    minify: false,
    assetsDir: "",
    cssCodeSplit: false,
    lib: {
      entry: "remoteEntry.ts",
      formats: ["es"],
      fileName: () => "remoteEntry.js",
    },
    rollupOptions: {
      external: ["react", "react-dom", "react/jsx-runtime", "react-dom/client"],
      output: {
        inlineDynamicImports: true,
      },
    },
  },
});
