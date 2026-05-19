import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      // The notes backend serves on :8080 by default; proxy /notes,
      // /auth, /mcp through vite during dev so cookies + CORS Just
      // Work without configuring CORS on the backend.
      "/notes": "http://localhost:8080",
      "/auth": "http://localhost:8080",
      "/mcp": "http://localhost:8080",
    },
  },
});
