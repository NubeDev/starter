import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";
import fs from "node:fs";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const FRONTEND_SRC = path.resolve(__dirname, "src");
const UI_KIT_SRC = path.resolve(
  __dirname,
  "../../../packages/starter-ui-kit/src",
);

// Mirrors the tsconfig `@/*` path fallback: prefer the frontend's own
// `src/*`, fall back to starter-ui-kit's `src/*`. Needed because vite's
// flat `resolve.alias` map cannot express an ordered fallback, and
// ui-kit's intra-package imports use `@/...` (which collide with the
// host's `@`). Without this, transitive ui-kit modules like
// `command.tsx` / `sidebar.tsx` / `toggle-group.tsx` fail to find
// their siblings (`input-group`, `collapsible`, `scroll-area`,
// `toggle`) at build time.
function aliasAtWithUiKitFallback(): Plugin {
  const exts = [".tsx", ".ts", ".jsx", ".js", ".mjs", ".cjs"];
  const resolveAgainst = (base: string, sub: string): string | null => {
    const direct = path.join(base, sub);
    if (fs.existsSync(direct) && fs.statSync(direct).isFile()) return direct;
    for (const ext of exts) {
      const candidate = direct + ext;
      if (fs.existsSync(candidate)) return candidate;
    }
    const index = path.join(direct, "index");
    for (const ext of exts) {
      const candidate = index + ext;
      if (fs.existsSync(candidate)) return candidate;
    }
    return null;
  };
  return {
    name: "flow-agent:alias-at-with-ui-kit-fallback",
    enforce: "pre",
    resolveId(source) {
      if (!source.startsWith("@/")) return null;
      const sub = source.slice(2);
      return resolveAgainst(FRONTEND_SRC, sub) ?? resolveAgainst(UI_KIT_SRC, sub);
    },
  };
}

// The flow-agent backend serves on :9741. Proxy /api so the SPA can
// talk to it without configuring CORS.
//
// Streaming routes (`/api/builder/stream`, SSE) need the proxy to
// pass frames through *unbuffered*. By default `http-proxy` will
// happily forward `content-encoding: gzip` from the upstream and
// some intermediaries cache or transform event streams; stripping
// the encoding header and forcing `cache-control: no-transform`
// guarantees the first frame reaches the browser inside the 200 ms
// budget pinned in PAGE-BUILDER-LIVE-FRONTEND.md §4.6.
const sseSafeProxy: import("vite").ProxyOptions = {
  target: "http://localhost:9741",
  changeOrigin: true,
  // `selfHandleResponse: false` is the default — we only need to
  // observe response headers, not rewrite the body.
  configure: (proxy) => {
    proxy.on("proxyRes", (proxyRes) => {
      proxyRes.headers["cache-control"] = "no-transform";
      // Drop any upstream `content-encoding` so the browser doesn't
      // wait for a complete gzip/br block before flushing frames.
      delete proxyRes.headers["content-encoding"];
      // Hint to upstream caches (e.g. nginx) — harmless if absent.
      proxyRes.headers["x-accel-buffering"] = "no";
    });
  },
};

export default defineConfig({
  plugins: [aliasAtWithUiKitFallback(), react(), tailwindcss()],
  resolve: {
    alias: {
      "@kit": UI_KIT_SRC,
    },
  },
  server: {
    port: 9742,
    proxy: {
      "/api": sseSafeProxy,
      // starter-prefs + starter-i18n REST surface served by the
      // flow-agent backend. Proxying here lets the SPA hit
      // `/v1/me/preferences`, `/v1/units`, `/v1/i18n/manifest`, and
      // `/v1/i18n/catalogs/...` without CORS configuration.
      "/v1": "http://localhost:9741",
      "/health": "http://localhost:9741",
      "/openapi.json": "http://localhost:9741",
    },
  },
});
