import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import cssInjectedByJsPlugin from "vite-plugin-css-injected-by-js";

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
        inlineDynamicImports: true,
      },
    },
  },
});
