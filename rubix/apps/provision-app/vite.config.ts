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
    // No /api proxy: the web transport talks to the agent's absolute URL
    // directly and authenticates with a Bearer token (see webTransport.ts), so
    // there's no same-origin cookie to preserve. The agent's permissive CORS
    // allows the cross-origin Authorization header. This is what lets the app
    // reach a remote agent over the internet, not just one on this machine.
  },
})
