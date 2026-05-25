import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { TanStackRouterVite } from '@tanstack/router-plugin/vite'
import path from 'node:path'

export default defineConfig({
  plugins: [
    TanStackRouterVite({ routesDirectory: './src/routes', generatedRouteTree: './src/routeTree.gen.ts' }),
    react(),
    tailwindcss(),
  ],
  resolve: {
    alias: { '@': path.resolve(__dirname, './src') },
  },
  server: {
    port: 5185,
    // Proxy REST + OpenAPI to the local rubix-agent so the SPA can use
    // same-origin paths in dev. The agent listens on 127.0.0.1:8088 by
    // default; override the upstream by exporting `VITE_RUBIX_BASE_URL`
    // and constructing `RubixClient` against that URL directly (see
    // `src/lib/client.ts`).
    proxy: {
      '/api/v1': { target: 'http://127.0.0.1:8088', changeOrigin: true },
      // ClickHouse explorer read-only sub-router mounted by
      // rubix-agent (`starter_warehouse::explorer::routes`). Powers
      // `/admin/warehouse` → Explorer tab via `@nube/starter-ui-ch-explorer`.
      '/api/warehouse': { target: 'http://127.0.0.1:8088', changeOrigin: true },
      '/openapi.json': { target: 'http://127.0.0.1:8088', changeOrigin: true },
    },
  },
})
