import { defineConfig } from "vite";

// Build configuration for the `com.nexus.hello` UI bundle.
//
// Same shape as `rubix/extensions/com.rubix.example/ui-src/vite.config.ts`,
// minus the Tailwind machinery (this panel uses inline styles, keeping the
// example's bundle minimal). We do NOT use a Module-Federation plugin — the
// nexus host expects the SDK-shape factory (`{ singletons, init(handle) }`)
// from `@nube/starter-ext-sdk-ts`; see
// `starter-extensions/packages/starter-ext-ui/src/host-manager.ts`.
//
// React and the JSX runtime are EXTERNAL. nexus-ui publishes its own React on
// `globalThis.__rubixReact*` and declares an importmap in `nexus/ui/index.html`
// pointing `react` / `react-dom` / `react/jsx-runtime` at shim modules under
// `/shims/*.mjs`. When the host dynamically `import()`s this bundle, the
// browser resolves those bare specifiers through the importmap to the host's
// React instance — bundling React would reintroduce the two-React-copies
// hook-mismatch error the host's singleton negotiation exists to prevent.
export default defineConfig({
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
