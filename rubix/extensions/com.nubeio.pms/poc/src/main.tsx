import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createHashRouter, RouterProvider } from "react-router-dom";
import { StoreProvider } from "@/store/store";
import { Shell } from "@/components/Shell";
import { Dashboard } from "@/pages/Dashboard";
import { ClientsPage } from "@/pages/ClientsPage";
import { TemplatesPage } from "@/pages/TemplatesPage";
import { ProjectsPage } from "@/pages/ProjectsPage";
import { ProjectBuilder } from "@/pages/ProjectBuilder";
import "./index.css";

const router = createHashRouter([
  {
    path: "/",
    element: <Shell />,
    children: [
      { index: true, element: <Dashboard /> },
      { path: "clients", element: <ClientsPage /> },
      { path: "templates", element: <TemplatesPage /> },
      { path: "projects", element: <ProjectsPage /> },
      { path: "projects/:projectId", element: <ProjectBuilder /> },
    ],
  },
]);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <StoreProvider>
      <RouterProvider router={router} />
    </StoreProvider>
  </StrictMode>,
);
