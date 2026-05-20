import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { extensionCatalogWatcher } from "./vite-plugin-i18n-watcher.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react(), tailwindcss(), extensionCatalogWatcher()],
  resolve: {
    alias: {
      // `@nube/starter-ui-kit` ships source-only and uses `@/` to
      // self-reference its component / lib directories. Vite needs
      // the alias spelled out so the import graph resolves at dev
      // time the same way tsc does (see tsconfig#paths).
      "@": path.resolve(
        __dirname,
        "../../../packages/starter-ui-kit/src",
      ),
    },
  },
  server: {
    port: 5173,
    proxy: {
      // The notes backend serves on :8080 by default; proxy /notes,
      // /auth, /mcp through vite during dev so cookies + CORS Just
      // Work without configuring CORS on the backend.
      "/notes": "http://localhost:8080",
      "/auth": "http://localhost:8080",
      "/mcp": "http://localhost:8080",
      "/extensions": "http://localhost:8080",
      "/health": "http://localhost:8080",
      "/hello": "http://localhost:8080",
      "/api": "http://localhost:8080",
    },
  },
});
