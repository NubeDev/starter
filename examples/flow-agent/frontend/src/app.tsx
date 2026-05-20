import { Navigate, Route, Routes } from "react-router-dom";

import { Shell } from "./layout/Shell";
import { AgentChat } from "./pages/AgentChat";
import { AgentsList } from "./pages/AgentsList";
import { FlowEditor } from "./pages/FlowEditor";
import { FlowsList } from "./pages/FlowsList";
import { Settings } from "./pages/Settings";

export function App() {
  return (
    <Routes>
      <Route element={<Shell />}>
        <Route index element={<Navigate to="/flows" replace />} />
        <Route path="/flows" element={<FlowsList />} />
        <Route path="/flows/:id" element={<FlowEditor />} />
        <Route path="/agents" element={<AgentsList />} />
        <Route path="/agents/:id" element={<AgentChat />} />
        <Route path="/settings" element={<Settings />} />
        <Route path="*" element={<Navigate to="/flows" replace />} />
      </Route>
    </Routes>
  );
}
