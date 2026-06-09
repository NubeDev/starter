import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Proxy /api to the Rust backend so the UI can use same-origin relative URLs.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5274,
    proxy: { "/api": "http://127.0.0.1:8787" },
  },
});
