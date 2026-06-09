// Shared AI-assist hook: a single mutation over POST /ai/assist used by the
// query editor (task "sql") and the dashboard builder (tasks "panel"/"dashboard").
// Synchronous — one structured artifact back, no streaming. Helpers below lift
// the typed `result` shapes the backend documents per task.
import { useMutation } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { aiAssist } from "@/api/ai";
import type { AssistRequest, AssistResponse } from "@/api/types";

/** Run one AI-assist request. Caller picks the task + supplies grounding. */
export function useAssist() {
  const client = useStarterClient();
  return useMutation<AssistResponse, Error, AssistRequest>({
    mutationFn: (req) => aiAssist(client, req),
  });
}

/** A suggested panel as returned in an assist `result` (task panel/dashboard). */
export interface SuggestedPanel {
  title: string;
  viz: string;
  sql: string;
  x?: string | null;
  value: string;
}

/** A suggested dashboard as returned in an assist `result` (task dashboard). */
export interface SuggestedDashboard {
  name: string;
  panels: SuggestedPanel[];
}

/** Lift the generated SQL from a task=sql assist result. */
export function resultSql(res: AssistResponse): string {
  const r = res.result as { sql?: unknown } | null;
  if (r && typeof r.sql === "string") return r.sql;
  return typeof res.raw === "string" ? res.raw : "";
}

/** Lift a panel suggestion from a task=panel assist result, or null if malformed. */
export function resultPanel(res: AssistResponse): SuggestedPanel | null {
  const r = res.result as Partial<SuggestedPanel> | null;
  if (r && typeof r.title === "string" && typeof r.sql === "string" && typeof r.value === "string") {
    return {
      title: r.title,
      viz: typeof r.viz === "string" ? r.viz : "table",
      sql: r.sql,
      x: typeof r.x === "string" ? r.x : null,
      value: r.value,
    };
  }
  return null;
}

/** Lift a dashboard suggestion from a task=dashboard assist result, or null. */
export function resultDashboard(res: AssistResponse): SuggestedDashboard | null {
  const r = res.result as { name?: unknown; panels?: unknown } | null;
  if (!r || typeof r.name !== "string" || !Array.isArray(r.panels)) return null;
  const panels = r.panels
    .map((p) => resultPanel({ task: "panel", result: p } as AssistResponse))
    .filter((p): p is SuggestedPanel => p !== null);
  return { name: r.name, panels };
}
