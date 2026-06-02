import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import cssInjectedByJsPlugin from "vite-plugin-css-injected-by-js";
import type { AcceptedPlugin } from "postcss";

// Build config — same recipe as `com.nubeio.rubixos`. The rubix host
// expects an SDK-shape factory (`{ singletons, init(handle) }`),
// React/ReactDOM externalised (importmap publishes the host's
// instances), and a single ESM bundle emitted at `ui/remoteEntry.js`.
//
// NOTE: `@nube/starter-ui-flow` + `@xyflow/react` are NOT externalised
// — they're pulled into the bundle so the wiresheet canvas works
// standalone (React is the only shared singleton).

const EXTENSION_ID = "com.nubeio.ce";
const SCOPE_SELECTOR = `[data-ext-id="${EXTENSION_ID}"]`;

/**
 * `scope-extension-css` — prefix every selector in the extension's
 * bundled CSS with `[data-ext-id="…"]` so the extension's utility
 * rules ONLY apply inside its own DOM subtree (see the long note in
 * `com.nubeio.rubixos/ui-src/vite.config.ts`).
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
  define: {
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
