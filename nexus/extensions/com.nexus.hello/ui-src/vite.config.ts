import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import cssInjectedByJsPlugin from "vite-plugin-css-injected-by-js";
import type { AcceptedPlugin } from "postcss";

// Build configuration for the `com.nexus.hello` UI bundle.
//
// This is the *robust* extension-styling recipe, ported from
// `rubix/extensions/com.nubeio.rubixos`. The earlier version of this file
// shipped no Tailwind at all and relied on the host's bundle happening to
// contain the classes the panel used — which silently failed for any class the
// host didn't itself use (`text-card-foreground`, arbitrary values, …). Instead
// the extension now ships its OWN Tailwind v4 bundle that scans its OWN source,
// so every shadcn/Tailwind utility it uses is guaranteed to be generated.
//
// Three pillars:
//   1. `@tailwindcss/vite` compiles the extension's `app.css` against the
//      extension's own source files (so every class it uses is emitted).
//   2. `vite-plugin-css-injected-by-js` folds that CSS into the JS bundle, so
//      the single `remoteEntry.js` the host loads carries its own styles — no
//      separate stylesheet request.
//   3. `scopeExtensionCssPlugin` prefixes every emitted selector with
//      `[data-ext-id="…"]` so the extension's utility rules apply ONLY inside
//      its own DOM subtree and never collide with the host's identically-named
//      Tailwind classes (see the long note on the plugin below). Every page
//      wraps its content in `<div data-ext-id="com.nexus.hello">`.
//
// The host integration constraints are unchanged: the host expects the
// SDK-shape factory (`{ singletons, init(handle) }`); React + the JSX runtime
// are EXTERNAL (the host publishes its own React via the importmap), and the
// output is a single ESM bundle at `../ui/remoteEntry.js`.

const EXTENSION_ID = "com.nexus.hello";
const SCOPE_SELECTOR = `[data-ext-id="${EXTENSION_ID}"]`;

/**
 * `scope-extension-css` — PostCSS plugin that prefixes every selector in the
 * extension's bundled CSS with `[data-ext-id="…"]` so the extension's utility
 * rules ONLY apply inside its own DOM subtree.
 *
 * Without scoping, the extension's bundle ships `.grid-cols-1`, `.flex`,
 * `.bg-card`, etc. — identical class names to the host's Tailwind bundle. Both
 * load on the same page, both target the same DOM nodes by class, and
 * source-order tiebreaks pick a different winner depending on which bundle
 * loaded last. Scoping at the selector level resolves it permanently:
 *
 *   .bg-card { … }
 *
 * becomes
 *
 *   [data-ext-id="com.nexus.hello"] .bg-card,
 *   [data-ext-id="com.nexus.hello"].bg-card { … }
 *
 * — the rule only matches a `.bg-card` element that is, or lives inside, an
 * element carrying the extension's data attribute. The host's `.bg-card` rule
 * is unchanged and authoritative everywhere else.
 *
 * Universal selectors (`*`, `:root`, `html`, `body`, keyframe steps, and @-rule
 * params) are left alone.
 */
function scopeExtensionCssPlugin(scope: string): AcceptedPlugin {
  const SKIP_RE = /^(\*|:root\b|:host\b|html\b|body\b|from\b|to\b|\d+%|@)/;

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
        // One file — the host loads exactly `remoteEntry.js`.
        inlineDynamicImports: true,
      },
    },
  },
});
