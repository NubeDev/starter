import { Navigate, Route, Routes } from "react-router-dom";

import { Shell } from "./layout/Shell";
import { AgentChat } from "./pages/AgentChat";
import { AgentsList } from "./pages/AgentsList";
import { CacheDemo } from "./pages/CacheDemo";
import { FlowEditor } from "./pages/FlowEditor";
import { FlowsList } from "./pages/FlowsList";
import { PageBuilder } from "./pages/PageBuilder";
import { PagesList } from "./pages/PagesList";
import { PageView } from "./pages/PageView";
import { PipelineCanvas } from "./pages/PipelineCanvas";
import { RuleEditor } from "./pages/RuleEditor";
import { RulesList } from "./pages/RulesList";
import { Settings } from "./pages/Settings";
import { Skills } from "./pages/Skills";
import { VerdictsView } from "./pages/VerdictsView";

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
        <Route path="/insights/rules" element={<RulesList />} />
        <Route path="/insights/rules/:id" element={<RuleEditor />} />
        <Route path="/insights/pipelines" element={<PipelineCanvas />} />
        <Route path="/insights/verdicts" element={<VerdictsView />} />
        <Route path="/insights/verdicts/:id" element={<VerdictsView />} />
        <Route path="/cache-demo" element={<CacheDemo />} />
        <Route path="/skills" element={<Skills />} />
        <Route path="/settings" element={<Settings />} />
        <Route path="*" element={<Navigate to="/flows" replace />} />
      </Route>
    </Routes>
  );
}
