import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Standalone harness for `@nube/ce-wiresheet`.
//
// Like the original flowTest ce-ui, the engine is reached through a same-origin
// Vite proxy rather than a direct cross-origin connection — so REST GETs carry
// no Origin header and need no CORS, and the editor talks to its own origin.
//
//   CE_REST_ORIGIN    — the engine to proxy to        (default 127.0.0.1:7878)
//   CE_ALLOWED_ORIGIN — Origin to present on the WS    (default 127.0.0.1:5173)
//
// The WS handshake always sends an Origin; many engines allow-list it. We
// rewrite it to an origin the engine already trusts (the rubix frontend's),
// so the harness works without touching the engine's config. Override either
// via env if your engine allows a different origin (or set it to `*`).
const CE = process.env.CE_REST_ORIGIN ?? "http://127.0.0.1:7878";
const ALLOWED_ORIGIN = process.env.CE_ALLOWED_ORIGIN ?? "http://127.0.0.1:5173";

export default defineConfig({
  plugins: [react()],
  // The workspace store holds several react/react-dom copies (other packages
  // pin 19.2.7); without deduping, vite's dep pre-bundle can grab a react-dom
  // that mismatches the workspace-pinned react@19.1.0 → "Incompatible React
  // versions". Force a single copy from this project's resolution.
  resolve: {
    dedupe: ["react", "react-dom", "react/jsx-runtime", "react-dom/client"],
  },
  server: {
    port: 5179,
    host: "0.0.0.0",
    allowedHosts: ["craig-desk-fedora", ".local"],
    proxy: {
      "/api": { target: CE, changeOrigin: true },
      "/ws": {
        target: CE,
        ws: true,
        changeOrigin: true,
        configure: (proxy) => {
          proxy.on("proxyReqWs", (proxyReq) => {
            proxyReq.setHeader("origin", ALLOWED_ORIGIN);
          });
        },
      },
    },
  },
});
