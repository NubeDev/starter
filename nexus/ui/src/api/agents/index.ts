// Agent + session API bindings. Agents are configured AI endpoints (backend +
// model + system prompt); sessions are conversations against them. CRUD is JSON
// over the cookie+CSRF transport like every other resource; the session SSE feed
// is opened separately via `agentSessionEventsUrl` (a browser EventSource can't
// set an auth header, so the signed token rides the query string — F5).
import { fetchJson, fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type {
  AgentDetail,
  AgentSummary,
  CreateAgentRequest,
  CreateSessionRequest,
  CreateSessionResponse,
  SessionDetail,
  UpdateAgentRequest,
} from "@/api/types";

/** `GET /api/v1/agents` — the caller's agents (viewable ones). */
export function listAgents(client: StarterClient): Promise<AgentSummary[]> {
  return fetchJson<AgentSummary[]>(client, `${client.apiPrefix}/agents`);
}

/** `GET /api/v1/agents/{id}` — one agent in full. */
export function getAgent(client: StarterClient, id: string): Promise<AgentDetail> {
  return fetchJson<AgentDetail>(
    client,
    `${client.apiPrefix}/agents/${encodeURIComponent(id)}`,
  );
}

/** `POST /api/v1/agents` — define an agent. */
export function createAgent(
  client: StarterClient,
  request: CreateAgentRequest,
): Promise<AgentDetail> {
  return fetchJson<AgentDetail>(client, `${client.apiPrefix}/agents`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}

/** `PUT /api/v1/agents/{id}` — edit an agent (partial). */
export function updateAgent(
  client: StarterClient,
  id: string,
  request: UpdateAgentRequest,
): Promise<AgentDetail> {
  return fetchJson<AgentDetail>(
    client,
    `${client.apiPrefix}/agents/${encodeURIComponent(id)}`,
    {
      method: "PUT",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}

/** `DELETE /api/v1/agents/{id}` — remove an agent (sessions cascade, 204). */
export async function deleteAgent(client: StarterClient, id: string): Promise<void> {
  await fetchVoid(client, `${client.apiPrefix}/agents/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: readCsrfHeader(),
  });
}

/** `POST /api/v1/agents/{id}/sessions` — open + start a session; returns the
 * session id, status, and a signed token for its SSE feed. */
export function createAgentSession(
  client: StarterClient,
  agentId: string,
  request: CreateSessionRequest,
): Promise<CreateSessionResponse> {
  return fetchJson<CreateSessionResponse>(
    client,
    `${client.apiPrefix}/agents/${encodeURIComponent(agentId)}/sessions`,
    {
      method: "POST",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}

/** `GET /api/v1/agents/{id}/sessions` — sessions for one agent. */
export function listAgentSessions(
  client: StarterClient,
  agentId: string,
): Promise<SessionDetail[]> {
  return fetchJson<SessionDetail[]>(
    client,
    `${client.apiPrefix}/agents/${encodeURIComponent(agentId)}/sessions`,
  );
}

/** `GET /api/v1/agents/sessions/{id}` — one session + its persisted transcript. */
export function getAgentSession(
  client: StarterClient,
  sessionId: string,
): Promise<SessionDetail> {
  return fetchJson<SessionDetail>(
    client,
    `${client.apiPrefix}/agents/sessions/${encodeURIComponent(sessionId)}`,
  );
}

/** The SSE URL for a session's live event feed. The token (from
 * `createAgentSession`) rides the query string. Pass to `streamJson` /
 * `EventSource`. */
export function agentSessionEventsUrl(
  client: StarterClient,
  sessionId: string,
  token: string,
): string {
  return `${client.apiPrefix}/agents/sessions/${encodeURIComponent(
    sessionId,
  )}/events?token=${encodeURIComponent(token)}`;
}
