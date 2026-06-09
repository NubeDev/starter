import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Route, Routes } from "react-router-dom";
import { Refine } from "@refinedev/core";
import routerBindings from "@refinedev/react-router-v6";
import { Toaster } from "sonner";

import { Layout } from "@/components/layout/Layout";
import { Index } from "@/pages/Index";
import { DashboardPage } from "@/pages/DashboardPage";
import { dataProvider } from "@/providers/dataProvider";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <BrowserRouter>
      <Refine
        dataProvider={dataProvider}
        routerProvider={routerBindings}
        resources={[
          {
            name: "dashboards",
            list: "/d",
            create: "/d/create",
            edit: "/d/:id/edit",
          },
        ]}
        options={{ syncWithLocation: false, warnWhenUnsavedChanges: false, disableTelemetry: true }}
      >
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<Index />} />
            <Route path="/d/:slug" element={<DashboardPage />} />
          </Route>
        </Routes>
        <Toaster
          position="top-right"
          theme="dark"
          toastOptions={{
            style: {
              background: "hsl(222 40% 8%)",
              border: "1px solid hsl(217 33% 18%)",
              color: "hsl(210 40% 98%)",
            },
          }}
        />
      </Refine>
    </BrowserRouter>
  </React.StrictMode>
);
