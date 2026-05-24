import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

// Bare-bones Vite shell for the host-provider smoke page. Port 5180
// to avoid colliding with test-ui-2/3 and the rubix frontend.
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
  server: { port: 5180 },
});
