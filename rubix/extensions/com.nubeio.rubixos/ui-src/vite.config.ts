import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import cssInjectedByJsPlugin from "vite-plugin-css-injected-by-js";

// Build config — same recipe as `com.rubix.example`. The rubix host
// expects an SDK-shape factory (`{ singletons, init(handle) }`),
// React/ReactDOM externalised (importmap publishes the host's
// instances on `globalThis.__rubixReact*`), and a single ESM bundle
// emitted at `ui/remoteEntry.js`.
export default defineConfig({
  plugins: [tailwindcss(), cssInjectedByJsPlugin()],
  define: {
    // Surfaces in the browser console + on `window.__rubixosBuild`
    // so you can verify the page is running the freshly-built
    // remoteEntry.js (and not a cached older one) before chasing
    // CSS / specificity ghosts.
    __BUILD_STAMP__: JSON.stringify(new Date().toISOString()),
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
