import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// Tauri expects a fixed dev port; envPrefix exposes TAURI_* to the frontend.
// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_'],
  server: {
    port: 1421,
    strictPort: true,
    // Bind to all interfaces so the dev UI is reachable from other devices on
    // the LAN (e.g. a phone over WiFi at http://<machine-ip>:1421). Vite 8 also
    // rejects requests whose Host header isn't localhost; `true` accepts any
    // host, which is what we want for LAN dev where the machine's DHCP IP can
    // change. (Dev-only server; not used in the bundled app.)
    host: true,
    allowedHosts: true,
    // Proxy /api to the rubix-agent so the browser sees a SINGLE origin
    // (localhost:1421). The agent's session cookie is `SameSite=Lax`, which
    // a browser will not store or send on a cross-site fetch — so a
    // cross-origin call (1421 → 127.0.0.1:8088) authenticates the login but
    // drops the cookie on every subsequent tool call ("no caller identity").
    // Same-origin via this proxy makes the cookie first-party and it sticks.
    // Override the target with VITE_AGENT_PROXY when the agent runs elsewhere.
    proxy: {
      '/api': {
        target: process.env.VITE_AGENT_PROXY ?? 'http://127.0.0.1:8088',
        changeOrigin: true,
      },
    },
  },
})
