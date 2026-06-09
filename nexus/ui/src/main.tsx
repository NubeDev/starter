import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider } from "react-router-dom";

import { AppProviders } from "@/app/providers";
import { router } from "@/app/router";
import { initTheme } from "@/theme";
import "@/index.css";

// Paint the persisted theme onto <html> before React mounts, so the
// first frame is already the right palette (no flash of the wrong mode).
initTheme();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppProviders>
      <RouterProvider router={router} />
    </AppProviders>
  </StrictMode>,
);
