// React Query hooks for the admin surfaces. Read hooks return
// `UseQueryResult`, write hooks return `UseMutationResult`. All
// of them grab the long-lived `StarterClient` from
// `<StarterClientProvider>`; nothing else carries network state.
//
// Query keys are namespaced under `["authz", …]` so a single
// `queryClient.invalidateQueries({ queryKey: ["authz"] })` flushes
// the entire admin view after coarse changes (e.g. role change of
// the current operator).

import { useStarterClient } from "@nube/starter-client-react";
import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";
import type {
  AddMemberBody,
  AddTeamMemberBody,
  AssignmentBody,
  AssignmentResponse,
  AssignmentsListResponse,
  CheckRequest,
  CheckResponse,
  CreateTeamBody,
  CreateTenantBody,
  DecisionsPage,
  DecisionsQuery,
  MembershipView,
  PatchMemberBody,
  PatchTenantBody,
  ResourcesListResponse,
  RuleBody,
  RuleResponse,
  RulesListResponse,
  TeamView,
  TenantView,
} from "@nube/starter-client-ts";

// ----------------------------------------------------------------- keys

export const authzKeys = {
  all: ["authz"] as const,
  tenants: () => ["authz", "tenants"] as const,
  tenant: (id: string) => ["authz", "tenants", id] as const,
  members: (tenantId: string) => ["authz", "tenants", tenantId, "members"] as const,
  teams: (tenantId: string) => ["authz", "tenants", tenantId, "teams"] as const,
  rules: () => ["authz", "rules"] as const,
  assignments: () => ["authz", "assignments"] as const,
  resources: () => ["authz", "resources"] as const,
  decisions: (q?: DecisionsQuery) => ["authz", "decisions", q ?? {}] as const,
};

// ----------------------------------------------------------------- tenants

export function useTenants(): UseQueryResult<TenantView[], Error> {
  const c = useStarterClient();
  return useQuery({
    queryKey: authzKeys.tenants(),
    queryFn: () => c.listTenants(),
  });
}

export function useTenant(id: string | null): UseQueryResult<TenantView, Error> {
  const c = useStarterClient();
  return useQuery({
    queryKey: authzKeys.tenant(id ?? ""),
    queryFn: () => c.getTenant(id ?? ""),
    enabled: !!id,
  });
}

export function useCreateTenant(): UseMutationResult<TenantView, Error, CreateTenantBody> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body) => c.createTenant(body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: authzKeys.tenants() });
    },
  });
}

export interface PatchTenantArgs {
  id: string;
  body: PatchTenantBody;
}

export function usePatchTenant(): UseMutationResult<TenantView, Error, PatchTenantArgs> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }) => c.patchTenant(id, body),
    onSuccess: (_t, { id }) => {
      qc.invalidateQueries({ queryKey: authzKeys.tenants() });
      qc.invalidateQueries({ queryKey: authzKeys.tenant(id) });
    },
  });
}

// ----------------------------------------------------------------- members

/** Members aren't exposed as a list endpoint server-side — the
 * `MembersPanel` derives them from a tenant + an external user
 * directory. We expose only the write hooks here; consumers needing
 * a list can either fetch from their own users API or list via
 * `listAuthzAssignments()` and filter. */
export interface AddMemberArgs {
  tenantId: string;
  body: AddMemberBody;
}

export function useAddTenantMember(): UseMutationResult<MembershipView, Error, AddMemberArgs> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ tenantId, body }) => c.addTenantMember(tenantId, body),
    onSuccess: (_m, { tenantId }) => {
      qc.invalidateQueries({ queryKey: authzKeys.members(tenantId) });
    },
  });
}

export interface PatchMemberArgs {
  tenantId: string;
  userId: string;
  body: PatchMemberBody;
}

export function usePatchTenantMember(): UseMutationResult<void, Error, PatchMemberArgs> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ tenantId, userId, body }) =>
      c.patchTenantMember(tenantId, userId, body),
    onSuccess: (_v, { tenantId }) => {
      qc.invalidateQueries({ queryKey: authzKeys.members(tenantId) });
    },
  });
}

export interface RemoveMemberArgs {
  tenantId: string;
  userId: string;
}

export function useRemoveTenantMember(): UseMutationResult<void, Error, RemoveMemberArgs> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ tenantId, userId }) => c.removeTenantMember(tenantId, userId),
    onSuccess: (_v, { tenantId }) => {
      qc.invalidateQueries({ queryKey: authzKeys.members(tenantId) });
    },
  });
}

// ----------------------------------------------------------------- teams

export function useTeams(tenantId: string | null): UseQueryResult<TeamView[], Error> {
  const c = useStarterClient();
  return useQuery({
    queryKey: authzKeys.teams(tenantId ?? ""),
    queryFn: () => c.listTeams(tenantId ?? ""),
    enabled: !!tenantId,
  });
}

export interface CreateTeamArgs {
  tenantId: string;
  body: CreateTeamBody;
}

export function useCreateTeam(): UseMutationResult<TeamView, Error, CreateTeamArgs> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ tenantId, body }) => c.createTeam(tenantId, body),
    onSuccess: (_t, { tenantId }) => {
      qc.invalidateQueries({ queryKey: authzKeys.teams(tenantId) });
    },
  });
}

export interface DeleteTeamArgs {
  tenantId: string;
  teamId: string;
}

export function useDeleteTeam(): UseMutationResult<void, Error, DeleteTeamArgs> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ tenantId, teamId }) => c.deleteTeam(tenantId, teamId),
    onSuccess: (_v, { tenantId }) => {
      qc.invalidateQueries({ queryKey: authzKeys.teams(tenantId) });
    },
  });
}

export interface AddTeamMemberArgs {
  tenantId: string;
  teamId: string;
  body: AddTeamMemberBody;
}

export function useAddTeamMember(): UseMutationResult<void, Error, AddTeamMemberArgs> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ tenantId, teamId, body }) =>
      c.addTeamMember(tenantId, teamId, body),
    onSuccess: (_v, { tenantId }) => {
      qc.invalidateQueries({ queryKey: authzKeys.teams(tenantId) });
    },
  });
}

export interface RemoveTeamMemberArgs {
  tenantId: string;
  teamId: string;
  userId: string;
}

export function useRemoveTeamMember(): UseMutationResult<void, Error, RemoveTeamMemberArgs> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ tenantId, teamId, userId }) =>
      c.removeTeamMember(tenantId, teamId, userId),
    onSuccess: (_v, { tenantId }) => {
      qc.invalidateQueries({ queryKey: authzKeys.teams(tenantId) });
    },
  });
}

// ----------------------------------------------------------------- rules

export function useAuthzRules(): UseQueryResult<RulesListResponse, Error> {
  const c = useStarterClient();
  return useQuery({
    queryKey: authzKeys.rules(),
    queryFn: () => c.listAuthzRules(),
  });
}

export function useCreateAuthzRule(): UseMutationResult<RuleResponse, Error, RuleBody> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body) => c.createAuthzRule(body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: authzKeys.rules() });
    },
  });
}

export interface UpdateRuleArgs {
  id: string;
  body: RuleBody;
}

export function useUpdateAuthzRule(): UseMutationResult<RuleResponse, Error, UpdateRuleArgs> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }) => c.updateAuthzRule(id, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: authzKeys.rules() });
    },
  });
}

export function useDeleteAuthzRule(): UseMutationResult<void, Error, string> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id) => c.deleteAuthzRule(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: authzKeys.rules() });
    },
  });
}

// ----------------------------------------------------------------- assignments

export function useAuthzAssignments(): UseQueryResult<AssignmentsListResponse, Error> {
  const c = useStarterClient();
  return useQuery({
    queryKey: authzKeys.assignments(),
    queryFn: () => c.listAuthzAssignments(),
  });
}

export function useCreateAuthzAssignment(): UseMutationResult<
  AssignmentResponse,
  Error,
  AssignmentBody
> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body) => c.createAuthzAssignment(body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: authzKeys.assignments() });
    },
  });
}

export function useDeleteAuthzAssignment(): UseMutationResult<void, Error, string> {
  const c = useStarterClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id) => c.deleteAuthzAssignment(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: authzKeys.assignments() });
    },
  });
}

// ----------------------------------------------------------------- resources

export function useAuthzResources(): UseQueryResult<ResourcesListResponse, Error> {
  const c = useStarterClient();
  return useQuery({
    queryKey: authzKeys.resources(),
    queryFn: () => c.listAuthzResources(),
    staleTime: 60_000,
  });
}

// ----------------------------------------------------------------- check (mutation)

export function useAuthzCheck(): UseMutationResult<CheckResponse, Error, CheckRequest> {
  const c = useStarterClient();
  return useMutation({
    mutationFn: (body) => c.checkAuthz(body),
  });
}

// ----------------------------------------------------------------- decisions

export function useAuthzDecisions(
  query?: DecisionsQuery,
): UseQueryResult<DecisionsPage, Error> {
  const c = useStarterClient();
  return useQuery({
    queryKey: authzKeys.decisions(query),
    queryFn: () => c.listAuthzDecisions(query),
  });
}
