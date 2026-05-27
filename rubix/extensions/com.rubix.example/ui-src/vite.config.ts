import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import cssInjectedByJsPlugin from "vite-plugin-css-injected-by-js";

// Build configuration for the `com.rubix.example` UI bundle.
//
// We do NOT use @originjs/vite-plugin-federation — that plugin
// produces a Module-Federation-shaped `remoteEntry.js`
// (`{ init(shareScope), get(module) }`) which is incompatible with
// the rubix host. The host expects the SDK-shape factory
// (`{ singletons, init(handle) }`) from @nube/starter-ext-sdk-ts —
// see `rubix/frontend/src/routes/extensions.tsx::loadUi` and
// `starter-extensions/packages/starter-ext-ui/src/host-manager.ts`.
//
// React, ReactDOM and the JSX runtime are EXTERNAL. The host
// publishes its own instances on `globalThis.__rubixReact*` and
// declares an importmap in `rubix/frontend/index.html` that points
// `react` / `react-dom` / `react/jsx-runtime` at shim modules under
// `/shims/*.mjs`. When the host dynamically `import()`s this
// extension's `remoteEntry.js`, the browser resolves those bare
// specifiers through the importmap to the host's React instance —
// so `useContext` in the SDK's `BlockShell` and the context the
// host renders with are the SAME instance. Bundling React would
// reintroduce the two-React-copies hook-mismatch error.
//
// Tailwind CSS is processed at build time by @tailwindcss/vite and
// injected into the JS bundle via vite-plugin-css-injected-by-js.
// The generated utilities reference the host's CSS custom properties
// (--background, --primary, etc.) so the extension inherits the
// host's theme automatically.
export default defineConfig({
  plugins: [tailwindcss(), cssInjectedByJsPlugin()],
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
        // Keep everything in one file — the host loads exactly
        // `remoteEntry.js` and a single dynamic import is cheaper
        // than chasing a manifest of chunks.
        inlineDynamicImports: true,
      },
    },
  },
});
