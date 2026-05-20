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
    throw new Error(detail);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

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
};
