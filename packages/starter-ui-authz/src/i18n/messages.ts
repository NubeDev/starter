// Localizable strings the package emits at runtime. The package
// is react-intl-free; hosts derive an `AuthzMessages` object from
// their own translation hook and pass it via `<AuthzI18nProvider>`
// (or per-panel `i18n?: Partial<AuthzMessages>` props).
//
// Visible-string ownership matches the pattern established by
// `starter-ui-flow`, `starter-ui-chat`, `starter-ui-ai-builder`.
// Hosts that want to override the labels for built-in roles or
// resource kinds use `roleLabels` / `resourceLabels`.

export interface AuthzMessages {
  /** Top-level shell. */
  shell: {
    title: string;
    /** Tab labels in the shell. */
    tabs: {
      tenants: string;
      teams: string;
      members: string;
      rules: string;
      assignments: string;
      resources: string;
      check: string;
      decisions: string;
    };
  };
  /** Generic action / column labels reused across panels. */
  common: {
    loading: string;
    empty: string;
    error: string;
    save: string;
    create: string;
    cancel: string;
    delete: string;
    confirmDelete: string;
    edit: string;
    refresh: string;
    /** Subject id column header. */
    subject: string;
    /** Role column header. */
    role: string;
    /** Action column header (resource action list). */
    action: string;
    /** Resource kind column header. */
    resource: string;
    /** Effect column header (allow / deny). */
    effect: string;
    /** Priority column header. */
    priority: string;
    /** Tenant column header. */
    tenant: string;
    /** Created-by column header. */
    createdBy: string;
    /** Timestamp column header. */
    at: string;
    /** Reason column header. */
    reason: string;
    /** "Allow" label. */
    allow: string;
    /** "Deny" label. */
    deny: string;
    /** "All" / "Any" placeholder for unscoped values. */
    any: string;
  };
  /** Tenants panel. */
  tenants: {
    title: string;
    description: string;
    columns: {
      slug: string;
      displayName: string;
      auditSample: string;
    };
    form: {
      slugLabel: string;
      slugPlaceholder: string;
      displayNameLabel: string;
      displayNamePlaceholder: string;
      submit: string;
    };
  };
  /** Members panel (tenant memberships). */
  members: {
    title: string;
    description: string;
    selectTenantPrompt: string;
    columns: {
      user: string;
    };
    form: {
      userIdLabel: string;
      userIdPlaceholder: string;
      roleLabel: string;
      submit: string;
    };
  };
  /** Teams panel. */
  teams: {
    title: string;
    description: string;
    selectTenantPrompt: string;
    columns: {
      slug: string;
      displayName: string;
      members: string;
    };
    form: {
      slugLabel: string;
      displayNameLabel: string;
      submit: string;
    };
    /** Team-member sub-list. */
    teamMembers: {
      title: string;
      userIdLabel: string;
      add: string;
      /** Button/label that opens the manage-members dialog. */
      manage: string;
      /** Remove a member from the team. */
      remove: string;
      /** Empty state when a team has no members. */
      empty: string;
    };
  };
  /** Rules panel. */
  rules: {
    title: string;
    description: string;
    form: {
      roleLabel: string;
      resourceLabel: string;
      actionsLabel: string;
      actionsPlaceholder: string;
      conditionLabel: string;
      conditionPlaceholder: string;
      effectLabel: string;
      priorityLabel: string;
      tenantLabel: string;
      tenantPlaceholderGlobal: string;
      submit: string;
    };
  };
  /** Assignments panel. */
  assignments: {
    title: string;
    description: string;
    form: {
      subjectLabel: string;
      subjectPlaceholder: string;
      roleLabel: string;
      submit: string;
    };
  };
  /** Resources panel. */
  resources: {
    title: string;
    description: string;
    columns: {
      kind: string;
      label: string;
      actions: string;
      ownership: string;
      tenantScoped: string;
    };
  };
  /** Check / dry-run panel. */
  check: {
    title: string;
    description: string;
    principalSubjectLabel: string;
    principalRoleLabel: string;
    actionLabel: string;
    resourceKindLabel: string;
    resourceIdLabel: string;
    resourceOwnerLabel: string;
    submit: string;
    decisionAllow: string;
    decisionDeny: string;
    matchedRule: string;
  };
  /** Decisions panel. */
  decisions: {
    title: string;
    description: string;
    filters: {
      tenantLabel: string;
      subjectLabel: string;
      effectLabel: string;
      apply: string;
      reset: string;
    };
    loadMore: string;
    endOfList: string;
  };
  /** Optional overrides for built-in role names. */
  roleLabels?: Record<string, string>;
  /** Optional overrides for resource kind labels keyed by `kind`. */
  resourceLabels?: Record<string, string>;
}

/** Default English messages. */
export const DEFAULT_AUTHZ_MESSAGES: AuthzMessages = {
  shell: {
    title: "Access Control",
    tabs: {
      tenants: "Tenants",
      teams: "Teams",
      members: "Members",
      rules: "Rules",
      assignments: "Assignments",
      resources: "Resources",
      check: "Check",
      decisions: "Decisions",
    },
  },
  common: {
    loading: "Loading…",
    empty: "Nothing here yet.",
    error: "Something went wrong.",
    save: "Save",
    create: "Create",
    cancel: "Cancel",
    delete: "Delete",
    confirmDelete: "Are you sure?",
    edit: "Edit",
    refresh: "Refresh",
    subject: "Subject",
    role: "Role",
    action: "Action",
    resource: "Resource",
    effect: "Effect",
    priority: "Priority",
    tenant: "Tenant",
    createdBy: "Created by",
    at: "When",
    reason: "Reason",
    allow: "Allow",
    deny: "Deny",
    any: "Any",
  },
  tenants: {
    title: "Tenants",
    description: "Organisational scopes. Each tenant owns its own members, teams, and rule subset.",
    columns: {
      slug: "Slug",
      displayName: "Name",
      auditSample: "Audit sample",
    },
    form: {
      slugLabel: "Slug",
      slugPlaceholder: "acme",
      displayNameLabel: "Display name",
      displayNamePlaceholder: "Acme Corp",
      submit: "Create tenant",
    },
  },
  members: {
    title: "Members",
    description: "Users with a role binding in this tenant.",
    selectTenantPrompt: "Select a tenant to manage its members.",
    columns: {
      user: "User",
    },
    form: {
      userIdLabel: "User id",
      userIdPlaceholder: "user-id or email",
      roleLabel: "Role",
      submit: "Add member",
    },
  },
  teams: {
    title: "Teams",
    description: "Named groups inside a tenant. Rules may target a team instead of a role.",
    selectTenantPrompt: "Select a tenant to manage its teams.",
    columns: {
      slug: "Slug",
      displayName: "Name",
      members: "Members",
    },
    form: {
      slugLabel: "Slug",
      displayNameLabel: "Display name",
      submit: "Create team",
    },
    teamMembers: {
      title: "Team members",
      userIdLabel: "User id",
      add: "Add",
      manage: "Manage members",
      remove: "Remove",
      empty: "No members yet.",
    },
  },
  rules: {
    title: "Rules",
    description: "Per-role and per-resource policy. Highest priority wins; deny overrides allow on ties.",
    form: {
      roleLabel: "Role",
      resourceLabel: "Resource",
      actionsLabel: "Actions",
      actionsPlaceholder: "read, write, *",
      conditionLabel: "Condition",
      conditionPlaceholder: "owner",
      effectLabel: "Effect",
      priorityLabel: "Priority",
      tenantLabel: "Tenant",
      tenantPlaceholderGlobal: "(global)",
      submit: "Create rule",
    },
  },
  assignments: {
    title: "Assignments",
    description: "Bind a subject (user id or glob) to a role. Roles inherit the rules above.",
    form: {
      subjectLabel: "Subject",
      subjectPlaceholder: "user-id or user-*",
      roleLabel: "Role",
      submit: "Create assignment",
    },
  },
  resources: {
    title: "Resources",
    description: "Every resource kind the engine knows. Unknown kinds default to deny.",
    columns: {
      kind: "Kind",
      label: "Label",
      actions: "Actions",
      ownership: "Ownership",
      tenantScoped: "Tenant-scoped",
    },
  },
  check: {
    title: "Dry-run check",
    description: "Preview what the engine would decide right now.",
    principalSubjectLabel: "Principal subject",
    principalRoleLabel: "Principal role",
    actionLabel: "Action",
    resourceKindLabel: "Resource kind",
    resourceIdLabel: "Resource id",
    resourceOwnerLabel: "Resource owner",
    submit: "Check",
    decisionAllow: "Allow",
    decisionDeny: "Deny",
    matchedRule: "Matched rule",
  },
  decisions: {
    title: "Decisions",
    description: "Paged read of the engine audit sink. Newest first.",
    filters: {
      tenantLabel: "Tenant",
      subjectLabel: "Subject",
      effectLabel: "Effect",
      apply: "Apply",
      reset: "Reset",
    },
    loadMore: "Load more",
    endOfList: "End of list.",
  },
};

/** Deep-merge a partial override on top of `DEFAULT_AUTHZ_MESSAGES`. */
export function mergeAuthzMessages(
  override: Partial<AuthzMessages> | undefined,
): AuthzMessages {
  if (!override) return DEFAULT_AUTHZ_MESSAGES;
  const d = DEFAULT_AUTHZ_MESSAGES;
  return {
    shell: {
      ...d.shell,
      ...(override.shell ?? {}),
      tabs: { ...d.shell.tabs, ...(override.shell?.tabs ?? {}) },
    },
    common: { ...d.common, ...(override.common ?? {}) },
    tenants: {
      ...d.tenants,
      ...(override.tenants ?? {}),
      columns: { ...d.tenants.columns, ...(override.tenants?.columns ?? {}) },
      form: { ...d.tenants.form, ...(override.tenants?.form ?? {}) },
    },
    members: {
      ...d.members,
      ...(override.members ?? {}),
      columns: { ...d.members.columns, ...(override.members?.columns ?? {}) },
      form: { ...d.members.form, ...(override.members?.form ?? {}) },
    },
    teams: {
      ...d.teams,
      ...(override.teams ?? {}),
      columns: { ...d.teams.columns, ...(override.teams?.columns ?? {}) },
      form: { ...d.teams.form, ...(override.teams?.form ?? {}) },
      teamMembers: {
        ...d.teams.teamMembers,
        ...(override.teams?.teamMembers ?? {}),
      },
    },
    rules: {
      ...d.rules,
      ...(override.rules ?? {}),
      form: { ...d.rules.form, ...(override.rules?.form ?? {}) },
    },
    assignments: {
      ...d.assignments,
      ...(override.assignments ?? {}),
      form: { ...d.assignments.form, ...(override.assignments?.form ?? {}) },
    },
    resources: {
      ...d.resources,
      ...(override.resources ?? {}),
      columns: { ...d.resources.columns, ...(override.resources?.columns ?? {}) },
    },
    check: { ...d.check, ...(override.check ?? {}) },
    decisions: {
      ...d.decisions,
      ...(override.decisions ?? {}),
      filters: { ...d.decisions.filters, ...(override.decisions?.filters ?? {}) },
    },
    roleLabels: override.roleLabels,
    resourceLabels: override.resourceLabels,
  };
}
