import { Navigate, Route, Routes } from "react-router-dom";

import { Shell } from "./layout/Shell";
import { AgentChat } from "./pages/AgentChat";
import { AgentsList } from "./pages/AgentsList";
import { FlowEditor } from "./pages/FlowEditor";
import { FlowsList } from "./pages/FlowsList";
import { PageBuilder } from "./pages/PageBuilder";
import { PagesList } from "./pages/PagesList";
import { PageView } from "./pages/PageView";
import { Settings } from "./pages/Settings";
import { Skills } from "./pages/Skills";

export function App() {
  return (
    <Routes>
      <Route element={<Shell />}>
        <Route index element={<Navigate to="/flows" replace />} />
        <Route path="/flows" element={<FlowsList />} />
        <Route path="/flows/:id" element={<FlowEditor />} />
        <Route path="/agents" element={<AgentsList />} />
        <Route path="/agents/:id" element={<AgentChat />} />
        <Route path="/pages" element={<PagesList />} />
        <Route path="/pages/new" element={<PageBuilder />} />
        <Route path="/pages/:id" element={<PageView />} />
        <Route path="/pages/:id/edit" element={<PageBuilder />} />
        <Route path="/skills" element={<Skills />} />
        <Route path="/settings" element={<Settings />} />
        <Route path="*" element={<Navigate to="/flows" replace />} />
      </Route>
    </Routes>
  );
}
