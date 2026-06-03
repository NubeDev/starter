import { createRoot } from "react-dom/client";

import { CeEditor } from "@nube/ce-wiresheet";

// By default the editor talks to this harness's own origin, and Vite proxies
// `/api` + `/ws` to the engine (no CORS — see vite.config.ts). Override with
// `?base=http://<ip>:<port>` to hit an engine directly (that engine must then
// allow this origin's CORS + WS Origin).
const params = new URLSearchParams(window.location.search);
const base = params.get("base") || window.location.origin;

const root = document.getElementById("root");
if (!root) throw new Error("missing #root");

createRoot(root).render(
  <div style={{ position: "fixed", inset: 0 }}>
    <CeEditor base={base} />
  </div>,
);
