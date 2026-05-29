// Resources panel — read-only enumeration of the engine's
// `ResourceRegistry`. Powers the `<RulesPanel>` resource picker
// transitively (via `useAuthzResources`) and is shown here so an
// operator can see which kinds rules may reference.

import { useAuthzMessages } from "../i18n/context.js";
import { mergeAuthzMessages, type AuthzMessages } from "../i18n/messages.js";
import { useAuthzResources } from "../hooks/index.js";
import { DataTable, StateRow, Td } from "./_common.js";

export interface ResourcesPanelProps {
  i18n?: Partial<AuthzMessages>;
}

export function ResourcesPanel({ i18n }: ResourcesPanelProps) {
  const ctx = useAuthzMessages();
  const m = i18n ? mergeAuthzMessages({ ...ctx, ...i18n }) : ctx;

  const list = useAuthzResources();

  return (
    <section className="grid gap-6">
      <header className="grid gap-1">
        <h2 className="text-base font-semibold tracking-tight">
          {m.resources.title}
        </h2>
        <p className="text-sm text-muted-foreground">
          {m.resources.description}
        </p>
      </header>

      {list.isLoading ? (
        <StateRow variant="loading">{m.common.loading}</StateRow>
      ) : list.error ? (
        <StateRow variant="error">{list.error.message || m.common.error}</StateRow>
      ) : (list.data?.resources.length ?? 0) === 0 ? (
        <StateRow variant="empty">{m.common.empty}</StateRow>
      ) : (
        <DataTable
          label={m.resources.title}
          headers={[
            m.resources.columns.kind,
            m.resources.columns.label,
            m.resources.columns.actions,
            m.resources.columns.ownership,
            m.resources.columns.tenantScoped,
          ]}
          rows={(list.data?.resources ?? []).map((r) => (
            <tr key={r.kind}>
              <Td><code className="text-xs">{r.kind}</code></Td>
              <Td>{m.resourceLabels?.[r.kind] ?? r.label}</Td>
              <Td><code className="text-xs">{r.actions.join(", ")}</code></Td>
              <Td>{r.ownership}</Td>
              <Td>{r.tenant_scoped ? "✓" : "—"}</Td>
            </tr>
          ))}
        />
      )}
    </section>
  );
}
