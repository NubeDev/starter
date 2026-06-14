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
    port: 4790,
    // Same-origin in dev: forward the Nexus control-plane REST surface,
    // SSE streams, and extension artifacts to `nexus-api`. Override the
    // upstream with `VITE_NEXUS_BASE_URL` (consumed in `api/client.ts`).
    // `nexus-api` binds 127.0.0.1:4780 (NEXUS_BIND in
    // backend/crates/nexus-api/src/main.rs); the cookie-session login route
    // lives at `/auth/*`, outside the `/api/v1` product surface, so it needs
    // its own proxy entry or login 404s through the dev server.
    proxy: {
      "/api/v1": { target: "http://127.0.0.1:4780", changeOrigin: true },
      "/auth": { target: "http://127.0.0.1:4780", changeOrigin: true },
      // The authz admin surface (grants, share-scope, resource instances) is
      // mounted at `/v1/authz/*` — outside `/api/v1` — so dashboard sharing 404s
      // through the dev server without its own proxy entry.
      "/v1": { target: "http://127.0.0.1:4780", changeOrigin: true },
    },
  },
});
