// `/v1/tenants/*` client methods. Tenants, memberships, and per-
// tenant teams + team memberships. Routes are admin-gated server
// side; this module is shape-only and applies no role checks.
//
// Wire types are hand-rolled here because the tenant/team routes
// are not part of the OpenAPI surface today (only `/auth/*` is).
// Keep these in sync with the Rust handlers in
// `crates/starter-auth-users/src/routes/tenants.rs`.

import { StarterClient } from "../client/client.js";
import { readCsrfHeader } from "../client/csrf.js";
import { fetchJson } from "../client/fetch_json.js";
import { fetchVoid } from "../client/fetch_void.js";

/** One of `"reader" | "writer" | "admin"`. */
export type TenantRole = "reader" | "writer" | "admin";

/** JSON view returned by `GET /v1/tenants` and `GET /v1/tenants/{id}`. */
export interface TenantView {
  id: string;
  slug: string;
  display_name: string;
  /** Per-tenant override of the audit-log allow-sample rate. */
  audit_allow_sample: number | null;
}

/** Body for `POST /v1/tenants`. */
export interface CreateTenantBody {
  slug: string;
  display_name: string;
}

/** Body for `PATCH /v1/tenants/{id}`. Slug is immutable. */
export interface PatchTenantBody {
  display_name?: string;
  /**
   * `{ value: n }` sets the override; `{ value: null }` clears it
   * back to the global default; omit to leave unchanged.
   */
  audit_allow_sample?: number | null;
}

/** Membership row. */
export interface MembershipView {
  tenant_id: string;
  user_id: string;
  role: TenantRole;
  /** Present on the list endpoint (joins the users table); omitted on
   * add/patch responses. A human-readable label for member pickers. */
  email?: string;
}

/** Body for `POST /v1/tenants/{id}/members`. */
export interface AddMemberBody {
  user_id: string;
  role: TenantRole;
}

/** Body for `PATCH /v1/tenants/{id}/members/{user_id}`. */
export interface PatchMemberBody {
  role: TenantRole;
}

/** Body for `POST /v1/tenants/{id}/users` — create a new account + add to tenant. */
export interface CreateUserBody {
  email: string;
  password: string;
  role: TenantRole;
}

/** Team row. */
export interface TeamView {
  id: string;
  tenant_id: string;
  slug: string;
  display_name: string;
}

/** Body for `POST /v1/tenants/{id}/teams`. */
export interface CreateTeamBody {
  slug: string;
  display_name: string;
}

/** Body for `POST /v1/tenants/{id}/teams/{team_id}/members`. */
export interface AddTeamMemberBody {
  user_id: string;
}

declare module "../client/client.js" {
  interface StarterClient {
    // ----- tenants
    listTenants(): Promise<TenantView[]>;
    createTenant(body: CreateTenantBody): Promise<TenantView>;
    getTenant(id: string): Promise<TenantView>;
    patchTenant(id: string, body: PatchTenantBody): Promise<TenantView>;
    // ----- members
    listTenantMembers(id: string): Promise<MembershipView[]>;
    addTenantMember(id: string, body: AddMemberBody): Promise<MembershipView>;
    createTenantUser(id: string, body: CreateUserBody): Promise<MembershipView>;
    patchTenantMember(
      id: string,
      userId: string,
      body: PatchMemberBody,
    ): Promise<void>;
    removeTenantMember(id: string, userId: string): Promise<void>;
    // ----- teams
    listTeams(id: string): Promise<TeamView[]>;
    createTeam(id: string, body: CreateTeamBody): Promise<TeamView>;
    deleteTeam(id: string, teamId: string): Promise<void>;
    // ----- team members
    addTeamMember(
      id: string,
      teamId: string,
      body: AddTeamMemberBody,
    ): Promise<void>;
    removeTeamMember(
      id: string,
      teamId: string,
      userId: string,
    ): Promise<void>;
  }
}

const JSON_HEADERS = { "content-type": "application/json" };

function mutHeaders(): HeadersInit {
  return { ...JSON_HEADERS, ...readCsrfHeader() };
}

StarterClient.prototype.listTenants = function listTenants(this: StarterClient): Promise<TenantView[]> {
  return fetchJson<TenantView[]>(this, `/v1/tenants`);
};

StarterClient.prototype.createTenant = function createTenant(
  this: StarterClient,
  body: CreateTenantBody,
): Promise<TenantView> {
  return fetchJson<TenantView>(this, `/v1/tenants`, {
    method: "POST",
    headers: mutHeaders(),
    body: JSON.stringify(body),
  });
};

StarterClient.prototype.getTenant = function getTenant(this: StarterClient, id: string): Promise<TenantView> {
  return fetchJson<TenantView>(this, `/v1/tenants/${encodeURIComponent(id)}`);
};

StarterClient.prototype.patchTenant = function patchTenant(
  this: StarterClient,
  id: string,
  body: PatchTenantBody,
): Promise<TenantView> {
  return fetchJson<TenantView>(this, `/v1/tenants/${encodeURIComponent(id)}`, {
    method: "PATCH",
    headers: mutHeaders(),
    body: JSON.stringify(body),
  });
};

StarterClient.prototype.listTenantMembers = function listTenantMembers(
  this: StarterClient,
  id: string,
): Promise<MembershipView[]> {
  return fetchJson<MembershipView[]>(
    this,
    `/v1/tenants/${encodeURIComponent(id)}/members`,
  );
};

StarterClient.prototype.addTenantMember = function addTenantMember(
  this: StarterClient,
  id: string,
  body: AddMemberBody,
): Promise<MembershipView> {
  return fetchJson<MembershipView>(
    this,
    `/v1/tenants/${encodeURIComponent(id)}/members`,
    {
      method: "POST",
      headers: mutHeaders(),
      body: JSON.stringify(body),
    },
  );
};

StarterClient.prototype.createTenantUser = function createTenantUser(
  this: StarterClient,
  id: string,
  body: CreateUserBody,
): Promise<MembershipView> {
  return fetchJson<MembershipView>(
    this,
    `/v1/tenants/${encodeURIComponent(id)}/users`,
    {
      method: "POST",
      headers: mutHeaders(),
      body: JSON.stringify(body),
    },
  );
};

StarterClient.prototype.patchTenantMember = async function patchTenantMember(
  this: StarterClient,
  id: string,
  userId: string,
  body: PatchMemberBody,
): Promise<void> {
  await fetchVoid(
    this,
    `/v1/tenants/${encodeURIComponent(id)}/members/${encodeURIComponent(userId)}`,
    {
      method: "PATCH",
      headers: mutHeaders(),
      body: JSON.stringify(body),
    },
  );
};

StarterClient.prototype.removeTenantMember = async function removeTenantMember(
  this: StarterClient,
  id: string,
  userId: string,
): Promise<void> {
  await fetchVoid(
    this,
    `/v1/tenants/${encodeURIComponent(id)}/members/${encodeURIComponent(userId)}`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
};

StarterClient.prototype.listTeams = function listTeams(this: StarterClient, id: string): Promise<TeamView[]> {
  return fetchJson<TeamView[]>(this, `/v1/tenants/${encodeURIComponent(id)}/teams`);
};

StarterClient.prototype.createTeam = function createTeam(
  this: StarterClient,
  id: string,
  body: CreateTeamBody,
): Promise<TeamView> {
  return fetchJson<TeamView>(this, `/v1/tenants/${encodeURIComponent(id)}/teams`, {
    method: "POST",
    headers: mutHeaders(),
    body: JSON.stringify(body),
  });
};

StarterClient.prototype.deleteTeam = async function deleteTeam(
  this: StarterClient,
  id: string,
  teamId: string,
): Promise<void> {
  await fetchVoid(
    this,
    `/v1/tenants/${encodeURIComponent(id)}/teams/${encodeURIComponent(teamId)}`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
};

StarterClient.prototype.addTeamMember = async function addTeamMember(
  this: StarterClient,
  id: string,
  teamId: string,
  body: AddTeamMemberBody,
): Promise<void> {
  await fetchVoid(
    this,
    `/v1/tenants/${encodeURIComponent(id)}/teams/${encodeURIComponent(teamId)}/members`,
    {
      method: "POST",
      headers: mutHeaders(),
      body: JSON.stringify(body),
    },
  );
};

StarterClient.prototype.removeTeamMember = async function removeTeamMember(
  this: StarterClient,
  id: string,
  teamId: string,
  userId: string,
): Promise<void> {
  await fetchVoid(
    this,
    `/v1/tenants/${encodeURIComponent(id)}/teams/${encodeURIComponent(teamId)}/members/${encodeURIComponent(userId)}`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
};
