import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// The flow-agent backend serves on :8090. Proxy /api so the SPA can
// talk to it without configuring CORS.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(
        __dirname,
        "../../../packages/starter-ui-kit/src",
      ),
    },
  },
  server: {
    port: 5174,
    proxy: {
      "/api": "http://localhost:8090",
      "/health": "http://localhost:8090",
      "/openapi.json": "http://localhost:8090",
    },
  },
});
