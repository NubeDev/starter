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
    port: 1420,
    strictPort: true,
    // Proxy /api to the rubix-agent so the browser sees a SINGLE origin
    // (localhost:1420). The agent's session cookie is `SameSite=Lax`, which
    // a browser will not store or send on a cross-site fetch — so a
    // cross-origin call (1420 → 127.0.0.1:8088) authenticates the login but
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
