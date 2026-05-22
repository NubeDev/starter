import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter } from "react-router-dom";
import { ThemeProvider } from "@kit/theme";

import "./globals.css";
import { App } from "./app";
import { DefaultThemeBridge } from "@/components/default-theme-bridge";
import { PrefsHostShell } from "./prefs-host";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { refetchOnWindowFocus: false, staleTime: 5_000 },
  },
});

const root = document.getElementById("root");
if (!root) throw new Error("missing #root in index.html");

createRoot(root).render(
  <StrictMode>
    <ThemeProvider defaultTheme="system">
      <DefaultThemeBridge />
      <QueryClientProvider client={queryClient}>
        <PrefsHostShell queryClient={queryClient}>
          <BrowserRouter>
            <App />
          </BrowserRouter>
        </PrefsHostShell>
      </QueryClientProvider>
    </ThemeProvider>
  </StrictMode>,
);
