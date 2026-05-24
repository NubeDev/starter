// `/v1/authz/*` client methods. Rules, assignments, resources,
// dry-run check, and the paged decisions audit feed.
//
// Routes are admin-gated and CSRF-protected server side; the
// mutating helpers echo the `starter_csrf` cookie back as
// `X-CSRF-Token` via `readCsrfHeader`.
//
// Wire types mirror the Rust handlers in
// `crates/starter-authz/src/routes/{rules,assignments,resources,check,decisions}.rs`.

import { StarterClient } from "../client/client.js";
import { readCsrfHeader } from "../client/csrf.js";
import { fetchJson } from "../client/fetch_json.js";
import { fetchVoid } from "../client/fetch_void.js";

// ---------------------------------------------------------------- rules

export type RuleEffect = "allow" | "deny";

export interface RuleView {
  id: string;
  role: string;
  resource: string;
  actions: string[];
  condition: string | null;
  effect: RuleEffect;
  priority: number;
  created_by: string;
  tenant_id: string | null;
}

export interface RuleBody {
  id?: string;
  role: string;
  resource: string;
  actions: string[];
  condition?: string | null;
  effect: RuleEffect;
  priority?: number;
  tenant_id?: string | null;
}

export interface RulesListResponse {
  rules: RuleView[];
}

export interface RuleResponse {
  rule: RuleView;
}

// ---------------------------------------------------------------- assignments

export interface AssignmentView {
  id: string;
  subject: string;
  role: string;
  created_by: string;
}

export interface AssignmentBody {
  id?: string;
  subject: string;
  role: string;
}

export interface AssignmentsListResponse {
  assignments: AssignmentView[];
}

export interface AssignmentResponse {
  assignment: AssignmentView;
}

// ---------------------------------------------------------------- resources

export type ResourceOwnership = "none" | "owner";

export interface ResourceSpec {
  kind: string;
  actions: string[];
  ownership: ResourceOwnership;
  tenant_scoped: boolean;
  label: string;
  description: string;
}

export interface ResourcesListResponse {
  resources: ResourceSpec[];
}

// ---------------------------------------------------------------- check

export interface CheckRequest {
  principal: {
    subject: string;
    role: "reader" | "writer" | "admin";
    extra?: unknown;
  };
  action: string;
  resource: {
    kind: string;
    id?: string | null;
    owner?: string | null;
  };
}

export interface CheckResponse {
  decision: "allow" | "deny";
  reason?: string;
  matched_rule?: string;
}

// ---------------------------------------------------------------- decisions

export interface DecisionView {
  /** RFC3339 timestamp. */
  at: string;
  tenant: string | null;
  subject: string;
  principal_role: string;
  action: string;
  kind: string;
  id: string | null;
  effect: RuleEffect;
  rule_id: string | null;
  reason: string | null;
}

export interface DecisionsPage {
  items: DecisionView[];
  /** RFC3339 cursor — pass back as `before`. `null` on last page. */
  next_before: string | null;
}

export interface DecisionsQuery {
  tenant?: string;
  subject?: string;
  effect?: RuleEffect;
  /** RFC3339 timestamp. */
  before?: string;
  /** Clamped server-side to `[1, 500]`; defaults to 100. */
  limit?: number;
}

// ---------------------------------------------------------------- augment

declare module "../client/client.js" {
  interface StarterClient {
    // rules
    listAuthzRules(): Promise<RulesListResponse>;
    createAuthzRule(body: RuleBody): Promise<RuleResponse>;
    updateAuthzRule(id: string, body: RuleBody): Promise<RuleResponse>;
    deleteAuthzRule(id: string): Promise<void>;
    // assignments
    listAuthzAssignments(): Promise<AssignmentsListResponse>;
    createAuthzAssignment(body: AssignmentBody): Promise<AssignmentResponse>;
    deleteAuthzAssignment(id: string): Promise<void>;
    // resources
    listAuthzResources(): Promise<ResourcesListResponse>;
    // dry-run
    checkAuthz(body: CheckRequest): Promise<CheckResponse>;
    // decisions
    listAuthzDecisions(query?: DecisionsQuery): Promise<DecisionsPage>;
  }
}

const JSON_HEADERS = { "content-type": "application/json" };

function mutHeaders(): HeadersInit {
  return { ...JSON_HEADERS, ...readCsrfHeader() };
}

StarterClient.prototype.listAuthzRules = function listAuthzRules(this: StarterClient): Promise<RulesListResponse> {
  return fetchJson<RulesListResponse>(this, `/v1/authz/rules`);
};

StarterClient.prototype.createAuthzRule = function createAuthzRule(
  this: StarterClient,
  body: RuleBody,
): Promise<RuleResponse> {
  return fetchJson<RuleResponse>(this, `/v1/authz/rules`, {
    method: "POST",
    headers: mutHeaders(),
    body: JSON.stringify(body),
  });
};

StarterClient.prototype.updateAuthzRule = function updateAuthzRule(
  this: StarterClient,
  id: string,
  body: RuleBody,
): Promise<RuleResponse> {
  return fetchJson<RuleResponse>(this, `/v1/authz/rules/${encodeURIComponent(id)}`, {
    method: "PUT",
    headers: mutHeaders(),
    body: JSON.stringify(body),
  });
};

StarterClient.prototype.deleteAuthzRule = async function deleteAuthzRule(
  this: StarterClient,
  id: string,
): Promise<void> {
  await fetchVoid(this, `/v1/authz/rules/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: readCsrfHeader(),
  });
};

StarterClient.prototype.listAuthzAssignments = function listAuthzAssignments(
  this: StarterClient,
): Promise<AssignmentsListResponse> {
  return fetchJson<AssignmentsListResponse>(this, `/v1/authz/assignments`);
};

StarterClient.prototype.createAuthzAssignment = function createAuthzAssignment(
  this: StarterClient,
  body: AssignmentBody,
): Promise<AssignmentResponse> {
  return fetchJson<AssignmentResponse>(this, `/v1/authz/assignments`, {
    method: "POST",
    headers: mutHeaders(),
    body: JSON.stringify(body),
  });
};

StarterClient.prototype.deleteAuthzAssignment = async function deleteAuthzAssignment(
  this: StarterClient,
  id: string,
): Promise<void> {
  await fetchVoid(this, `/v1/authz/assignments/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: readCsrfHeader(),
  });
};

StarterClient.prototype.listAuthzResources = function listAuthzResources(
  this: StarterClient,
): Promise<ResourcesListResponse> {
  return fetchJson<ResourcesListResponse>(this, `/v1/authz/resources`);
};

StarterClient.prototype.checkAuthz = function checkAuthz(
  this: StarterClient,
  body: CheckRequest,
): Promise<CheckResponse> {
  return fetchJson<CheckResponse>(this, `/v1/authz/check`, {
    method: "POST",
    headers: JSON_HEADERS,
    body: JSON.stringify(body),
  });
};

StarterClient.prototype.listAuthzDecisions = function listAuthzDecisions(
  this: StarterClient,
  query?: DecisionsQuery,
): Promise<DecisionsPage> {
  const qs = new URLSearchParams();
  if (query?.tenant) qs.set("tenant", query.tenant);
  if (query?.subject) qs.set("subject", query.subject);
  if (query?.effect) qs.set("effect", query.effect);
  if (query?.before) qs.set("before", query.before);
  if (query?.limit !== undefined) qs.set("limit", String(query.limit));
  const suffix = qs.toString();
  return fetchJson<DecisionsPage>(
    this,
    suffix ? `/v1/authz/decisions?${suffix}` : `/v1/authz/decisions`,
  );
};
