// Hand-written REST client for flow-agent. Once OpenAPI codegen runs
// against /openapi.json, swap to `@nube/starter-client-ts/generated`.
// Until then, plain fetch is fine — the surface is small.

export type FlowSummary = {
  id: string;
  name: string;
  description: string | null;
  version: number;
  updated_at: string;
};

export type Flow = FlowSummary & {
  graph: FlowGraph;
  created_at: string;
};

export type FlowGraph = {
  nodes: FlowNode[];
  edges: FlowEdge[];
};

export type FlowNode = {
  id: string;
  kind: string;
  position: { x: number; y: number };
  data?: Record<string, unknown>;
};

export type FlowEdge = {
  id: string;
  source: string;
  sourceSlot: string;
  target: string;
  targetSlot: string;
};

export type CreateFlow = {
  name: string;
  description?: string;
  graph?: FlowGraph;
};

export type UpdateFlow = {
  name: string;
  description?: string | null;
  graph: FlowGraph;
  version: number;
};

export type AgentSummary = {
  id: string;
  name: string;
  provider: string;
  model: string;
  updated_at: string;
};

export type Agent = AgentSummary & {
  system_prompt: string | null;
  tools: string[];
  created_at: string;
};

export type CreateAgent = {
  name: string;
  provider: string;
  model: string;
  system_prompt?: string;
  tools?: string[];
};

export type UpdateAgent = CreateAgent;

export type Run = {
  id: string;
  flow_id: string;
  status: string;
  started_at: string;
  finished_at: string | null;
  trace: unknown;
};

export class ApiError extends Error {
  readonly status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

async function req<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(path, {
    method,
    headers: body !== undefined ? { "content-type": "application/json" } : {},
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    let detail = `${res.status} ${res.statusText}`;
    try {
      const json = await res.json();
      if (json.error) detail = json.error;
    } catch {
      /* ignore */
    }
    throw new ApiError(res.status, detail);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export type ProviderStatus = {
  id: string;
  label: string;
  available: boolean;
  hint: string;
};

export const api = {
  flows: {
    list: () => req<FlowSummary[]>("GET", "/api/flows"),
    get: (id: string) => req<Flow>("GET", `/api/flows/${id}`),
    create: (body: CreateFlow) => req<Flow>("POST", "/api/flows", body),
    update: (id: string, body: UpdateFlow) =>
      req<Flow>("PUT", `/api/flows/${id}`, body),
    delete: (id: string) => req<void>("DELETE", `/api/flows/${id}`),
    fire: (id: string, payload: unknown = {}) =>
      req<{ run_id: string }>("POST", `/api/flows/${id}/fire`, { payload }),
    runs: (id: string) => req<Run[]>("GET", `/api/flows/${id}/runs`),
  },
  agents: {
    list: () => req<AgentSummary[]>("GET", "/api/agents"),
    get: (id: string) => req<Agent>("GET", `/api/agents/${id}`),
    create: (body: CreateAgent) => req<Agent>("POST", "/api/agents", body),
    update: (id: string, body: UpdateAgent) =>
      req<Agent>("PUT", `/api/agents/${id}`, body),
    delete: (id: string) => req<void>("DELETE", `/api/agents/${id}`),
  },
  providers: {
    list: () => req<ProviderStatus[]>("GET", "/api/providers"),
  },
  insights: {
    listRules: () => req<InsightsRule[]>("GET", "/api/insights/rules"),
    getRule: (id: string) =>
      req<InsightsRule>("GET", `/api/insights/rules/${encodeURIComponent(id)}`),
    listVerdicts: (q: InsightsVerdictFilter = {}) => {
      const params = new URLSearchParams();
      if (q.rule_id) params.set("rule_id", q.rule_id);
      if (q.tag) params.set("tag", q.tag);
      if (q.severity) params.set("severity", q.severity);
      if (q.since) params.set("since", q.since);
      if (q.until) params.set("until", q.until);
      const qs = params.toString();
      return req<InsightsVerdict[]>(
        "GET",
        `/api/insights/verdicts${qs ? `?${qs}` : ""}`,
      );
    },
    getVerdict: (id: string) =>
      req<InsightsVerdict>(
        "GET",
        `/api/insights/verdicts/${encodeURIComponent(id)}`,
      ),
    listPipelines: () =>
      req<InsightsPipeline[]>("GET", "/api/insights/pipelines"),
    updateRule: (id: string, patch: Partial<InsightsRule>) =>
      req<InsightsRule>(
        "PATCH",
        `/api/insights/rules/${encodeURIComponent(id)}`,
        patch,
      ),
    dryRunRule: (id: string, input: unknown = {}) =>
      req<InsightsVerdict & { dry_run: true }>(
        "POST",
        `/api/insights/rules/${encodeURIComponent(id)}/dry-run`,
        input,
      ),
  },
};

// ---------------------------------------------------------------------
// Insights mock-up types — shapes mirror the eventual `starter-insights`
// schema (see DOCS/Insights/SCOPE.md). Kept structural here because the
// backend is still untyped serde_json::Value.
// ---------------------------------------------------------------------

export type InsightsSeverity =
  | "Healthy"
  | "Info"
  | "Warn"
  | "Critical"
  | "Error";

export type InsightsRule = {
  id: string;
  kind: string;
  namespace: string;
  severity_default: InsightsSeverity;
  tags: string[];
  summary: string;
  body: string;
  schema: Record<string, unknown>;
  created_at: string;
  updated_at: string;
};

export type InsightsCoverage = {
  raw: {
    samples_expected: number;
    samples_present: number;
    confidence: number;
  };
  effective: {
    confidence: number;
    penalty_chain: Array<[string, number]>;
  };
  quality_flags: Array<{
    id: string;
    severity: "Info" | "Warn" | "Critical";
    detail?: string;
  }>;
};

export type InsightsVerdict = {
  id: string;
  rule_id: string;
  at: string;
  tz: string;
  window: { start: string; end: string };
  severity: InsightsSeverity;
  coverage: InsightsCoverage;
  tags: string[];
  summary: string;
  evidence: Array<Record<string, unknown>>;
  ai_explanation?: string;
  correlation_id?: string;
};

export type InsightsVerdictFilter = {
  rule_id?: string;
  tag?: string;
  severity?: InsightsSeverity;
  since?: string;
  until?: string;
};

export type InsightsPipeline = {
  id: string;
  name: string;
  description?: string;
  tags: string[];
  graph: {
    nodes: Array<{
      id: string;
      kind: string;
      x: number;
      y: number;
      rule_id?: string;
    }>;
    edges: Array<{ from: string; to: string; type: string }>;
  };
  updated_at: string;
};
