// Browser entry. The dev shell mounts a single route — the
// extensions page — so the visual smoke for Phase D.2 is one URL
// away (`/`). Real router wiring lives downstream; this package is a
// smoke shell, not a product.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import ExtensionsPage from "./app/extensions/page.js";

const root = document.getElementById("root");
if (!root) {
  throw new Error("test-ui-5: #root container is missing from index.html");
}

createRoot(root).render(
  <StrictMode>
    <ExtensionsPage />
  </StrictMode>,
);
