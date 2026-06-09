import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

// Federation HOST config. Extension `remoteEntry.js` bundles externalise
// `react`/`react-dom`/`react/jsx-runtime` and resolve them through the
// importmap in `index.html`, which forwards to the shims under
// `public/shims/*`. Those shims read the React instance the host
// publishes on `globalThis.__rubixReact*` (see `app/providers.tsx`), so
// host and remotes share one React — a hard federation requirement.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
  server: {
    port: 5273,
    // Same-origin in dev: forward the Nexus control-plane REST surface,
    // SSE streams, and extension artifacts to `nexus-api`. Override the
    // upstream with `VITE_NEXUS_BASE_URL` (consumed in `api/client.ts`).
    proxy: {
      "/api/v1": { target: "http://127.0.0.1:8099", changeOrigin: true },
    },
  },
});
