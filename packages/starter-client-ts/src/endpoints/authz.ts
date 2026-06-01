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

// ---------------------------------------------------------------- resource instances

export type SubjectRef =
  | { kind: "team"; slug: string }
  | { kind: "user"; sub: string }
  | { kind: "wildcard" };

export type PermissionTier = "View" | "Edit" | "Manage";

export interface GrantSummary {
  subject: SubjectRef;
  tier: PermissionTier;
}

export type ShareScope = "private" | "tenant" | "specific";

export interface EffectiveAcl {
  share_scope: ShareScope;
  grants: GrantSummary[];
  has_legacy_rules: boolean;
}

export interface ResourceInstance {
  id: string;
  label: string;
  owner?: SubjectRef;
  updated_at?: string;
  effective_acl: EffectiveAcl;
}

export interface InstancesPage {
  items: ResourceInstance[];
  next_cursor?: string;
}

export interface InstancesQuery {
  search?: string;
  cursor?: string;
  /** Clamped server-side to `[1, 200]`; defaults to 50. */
  limit?: number;
  /** Override tenant scope; admins with global scope only. */
  tenant?: string;
}

// ---------------------------------------------------------------- grants (G3)

/** Subject of a grant. Matches the Rust `GrantSubject` enum. */
export type GrantSubject =
  | { kind: "team"; slug: string }
  | { kind: "user"; sub: string }
  | { kind: "wildcard" };

/** Server-side view of a grant — the rule row in a typed shape. */
export interface Grant {
  id: string;
  subject: GrantSubject;
  resource_kind: string;
  resource_id: string | null;
  tier: PermissionTier;
  tenant_id: string;
}

/** Body for `POST /v1/authz/grants`. */
export interface NewGrantBody {
  subject: GrantSubject;
  resource_kind: string;
  resource_id?: string | null;
  tier: PermissionTier;
  tenant_id: string;
}

/** Body for `PATCH /v1/authz/grants/:id`. */
export interface PatchGrantBody {
  tier: PermissionTier;
}

/** Body for `PUT /v1/authz/grants/share-scope/:kind/:resource_id`. */
export interface SetShareScopeBody {
  scope: ShareScope;
  /** Required only for super-admin callers. */
  tenant_id?: string;
}

export interface GrantsListResponse {
  grants: Grant[];
}

export interface GrantResponse {
  grant: Grant;
}

export interface ListGrantsQuery {
  subject?: string;
  resource_kind?: string;
  resource_id?: string;
  tenant_id?: string;
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
    listResourceInstances(
      kind: string,
      opts?: InstancesQuery,
    ): Promise<InstancesPage>;
    // dry-run
    checkAuthz(body: CheckRequest): Promise<CheckResponse>;
    // decisions
    listAuthzDecisions(query?: DecisionsQuery): Promise<DecisionsPage>;
    // grants (G3)
    createGrant(body: NewGrantBody): Promise<GrantResponse>;
    deleteGrant(id: string): Promise<void>;
    listGrants(query?: ListGrantsQuery): Promise<GrantsListResponse>;
    patchGrant(id: string, body: PatchGrantBody): Promise<GrantResponse>;
    setShareScope(
      kind: string,
      resourceId: string,
      body: SetShareScopeBody,
    ): Promise<void>;
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

StarterClient.prototype.listResourceInstances = function listResourceInstances(
  this: StarterClient,
  kind: string,
  opts?: InstancesQuery,
): Promise<InstancesPage> {
  const qs = new URLSearchParams();
  if (opts?.search) qs.set("search", opts.search);
  if (opts?.cursor) qs.set("cursor", opts.cursor);
  if (opts?.limit !== undefined) qs.set("limit", String(opts.limit));
  if (opts?.tenant) qs.set("tenant", opts.tenant);
  const suffix = qs.toString();
  const path = `/v1/authz/resources/${encodeURIComponent(kind)}/instances`;
  return fetchJson<InstancesPage>(this, suffix ? `${path}?${suffix}` : path);
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

StarterClient.prototype.createGrant = function createGrant(
  this: StarterClient,
  body: NewGrantBody,
): Promise<GrantResponse> {
  return fetchJson<GrantResponse>(this, `/v1/authz/grants`, {
    method: "POST",
    headers: mutHeaders(),
    body: JSON.stringify(body),
  });
};

StarterClient.prototype.deleteGrant = async function deleteGrant(
  this: StarterClient,
  id: string,
): Promise<void> {
  await fetchVoid(this, `/v1/authz/grants/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: readCsrfHeader(),
  });
};

StarterClient.prototype.listGrants = function listGrants(
  this: StarterClient,
  query?: ListGrantsQuery,
): Promise<GrantsListResponse> {
  const qs = new URLSearchParams();
  if (query?.subject) qs.set("subject", query.subject);
  if (query?.resource_kind) qs.set("resource_kind", query.resource_kind);
  if (query?.resource_id) qs.set("resource_id", query.resource_id);
  if (query?.tenant_id) qs.set("tenant_id", query.tenant_id);
  const suffix = qs.toString();
  return fetchJson<GrantsListResponse>(
    this,
    suffix ? `/v1/authz/grants?${suffix}` : `/v1/authz/grants`,
  );
};

StarterClient.prototype.patchGrant = function patchGrant(
  this: StarterClient,
  id: string,
  body: PatchGrantBody,
): Promise<GrantResponse> {
  return fetchJson<GrantResponse>(
    this,
    `/v1/authz/grants/${encodeURIComponent(id)}`,
    {
      method: "PATCH",
      headers: mutHeaders(),
      body: JSON.stringify(body),
    },
  );
};

StarterClient.prototype.setShareScope = async function setShareScope(
  this: StarterClient,
  kind: string,
  resourceId: string,
  body: SetShareScopeBody,
): Promise<void> {
  await fetchVoid(
    this,
    `/v1/authz/grants/share-scope/${encodeURIComponent(kind)}/${encodeURIComponent(resourceId)}`,
    {
      method: "PUT",
      headers: mutHeaders(),
      body: JSON.stringify(body),
    },
  );
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
