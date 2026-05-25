import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// The ch-explorer Rust binary serves this bundle under
// /warehouse/explorer/ via `ServerBuilder::with_static_assets`. Vite
// needs the same base at build time so generated asset URLs resolve
// in production. Override with `CH_EXPLORER_BASE` when mounting
// somewhere else.
const base = process.env.CH_EXPLORER_BASE ?? "/warehouse/explorer/";

// Local dev: vite on :5173, ch-explorer Rust binary on :3030. The
// library's API hooks call `/api/warehouse/ch/*` against the ambient
// StarterClient base URL; proxying keeps cookies + CORS out of the
// picture during dev.
const apiTarget = process.env.CH_EXPLORER_API_TARGET ?? "http://localhost:3030";

export default defineConfig({
  base,
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      "/api/warehouse": apiTarget,
    },
  },
});
