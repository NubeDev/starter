import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

// Tiny dev harness — `pnpm --filter @nube/starter-ui-sdui-puck run
// harness` opens a Vite dev server at http://localhost:5180/ with a
// `<PuckBuilder>` mounted over a hand-authored ComponentTree.
//
// Scope §"Definition of done": "One Storybook-or-equivalent harness
// page that mounts <PuckBuilder> … and lets you drag in one new
// widget from the palette. No save, no liveness, just the canvas
// working."

export default defineConfig({
  root: resolve(__dirname),
  plugins: [react()],
  server: { port: 5180, strictPort: true },
  build: { outDir: resolve(__dirname, "dist") },
});
