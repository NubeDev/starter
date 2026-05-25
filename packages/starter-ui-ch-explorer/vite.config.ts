// Forked from sql-studio (MIT) — https://github.com/frectonz/sql-studio
// Original copyright (c) frectonz. See NOTICES.md.

import path from "path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";

// Public asset mount. The starter-server serves the built dist
// under `/warehouse/explorer/` by default (see
// `crates/starter-server/src/static_assets.rs`). Override at build
// time via `CH_EXPLORER_BASE`.
const BASE = process.env.CH_EXPLORER_BASE ?? "/warehouse/explorer/";

// Local backend used by `pnpm dev`. Override via `CH_EXPLORER_API_TARGET`.
const API_TARGET =
  process.env.CH_EXPLORER_API_TARGET ?? "http://localhost:3030";

// https://vitejs.dev/config/
export default defineConfig({
  base: BASE,
  plugins: [
    tanstackRouter({
      target: "react",
      autoCodeSplitting: true,
    }),
    react(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      "@/": `${path.resolve(__dirname, "src")}/`,
    },
  },
  server: {
    proxy: {
      "/api/warehouse/ch": {
        target: API_TARGET,
        changeOrigin: true,
      },
    },
  },
});
