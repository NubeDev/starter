// App shell: sidebar + routed page (simple local route state, no router dep).

import { useState } from "react";
import { fetchBuffers, fetchInputs, fetchOutputs, fetchProcessors } from "./api/catalog";
import { Sidebar, type Route } from "./components/Sidebar";
import { BuilderPage } from "./pages/builder/BuilderPage";
import { CatalogPage } from "./pages/CatalogPage";
import { PluginsPage } from "./pages/PluginsPage";
import { SqlPage } from "./pages/SqlPage";

export function App() {
  const [route, setRoute] = useState<Route>("builder");

  return (
    <div className="shell">
      <Sidebar active={route} onNavigate={setRoute} />
      <main className="content">{renderPage(route)}</main>
    </div>
  );
}

function renderPage(route: Route) {
  switch (route) {
    case "builder":
      return <BuilderPage />;
    case "sql":
      return <SqlPage />;
    case "plugins":
      return <PluginsPage />;
    case "inputs":
      return <CatalogPage title="Inputs" blurb="Sources that feed a stream." load={fetchInputs} />;
    case "outputs":
      return <CatalogPage title="Outputs" blurb="Sinks a stream writes to." load={fetchOutputs} />;
    case "processors":
      return <CatalogPage title="Processors" blurb="Transform steps in the pipeline." load={fetchProcessors} />;
    case "buffers":
      return <CatalogPage title="Buffers" blurb="Batching / windowing between input and pipeline." load={fetchBuffers} />;
  }
}
