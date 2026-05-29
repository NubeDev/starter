// Decisions panel — paged read of `/v1/authz/decisions`. Newest
// first; `next_before` is forwarded to a "load more" button that
// appends to the rendered list.
//
// We keep paging state local instead of using
// `useInfiniteQuery` to stay dependency-light (consumers can swap
// in their own infinite-scroll wrapper around `useAuthzDecisions`
// if they want it).

import { useEffect, useState } from "react";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Input,
  Label,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit";
import { useStarterClient } from "@nube/starter-client-react";
import { useAuthzMessages } from "../i18n/context.js";
import { mergeAuthzMessages, type AuthzMessages } from "../i18n/messages.js";
import type { DecisionView, DecisionsQuery, RuleEffect } from "@nube/starter-client-ts";
import { useAuthzDecisions } from "../hooks/index.js";
import { DataTable, StateRow, Td } from "./_common.js";

export interface DecisionsPanelProps {
  i18n?: Partial<AuthzMessages>;
  /** Prefill the tenant filter (string — accepts id or slug — passed through to API). */
  tenantId?: string | null;
  /** Prefill the subject filter. */
  subject?: string | null;
}

export function DecisionsPanel({ i18n, tenantId: scopeTenant, subject: scopeSubject }: DecisionsPanelProps) {
  const ctx = useAuthzMessages();
  const m = i18n ? mergeAuthzMessages({ ...ctx, ...i18n }) : ctx;

  const client = useStarterClient();

  const [filters, setFilters] = useState<DecisionsQuery>(() => ({
    limit: 100,
    tenant: scopeTenant || undefined,
    subject: scopeSubject || undefined,
  }));
  const [tenant, setTenant] = useState(scopeTenant ?? "");
  const [subject, setSubject] = useState(scopeSubject ?? "");

  // Reapply when master-detail scope changes.
  useEffect(() => {
    setTenant(scopeTenant ?? "");
    setSubject(scopeSubject ?? "");
    setFilters({
      limit: 100,
      tenant: scopeTenant || undefined,
      subject: scopeSubject || undefined,
    });
  }, [scopeTenant, scopeSubject]);
  const [effect, setEffect] = useState<"" | RuleEffect>("");
  const [extra, setExtra] = useState<DecisionView[]>([]);
  const [nextBefore, setNextBefore] = useState<string | null>(null);
  const [loadingMore, setLoadingMore] = useState(false);

  const list = useAuthzDecisions(filters);

  function onApply() {
    setExtra([]);
    setNextBefore(null);
    setFilters({
      limit: 100,
      tenant: tenant.trim() || undefined,
      subject: subject.trim() || undefined,
      effect: effect || undefined,
    });
  }

  function onReset() {
    setTenant("");
    setSubject("");
    setEffect("");
    setExtra([]);
    setNextBefore(null);
    setFilters({ limit: 100 });
  }

  async function onLoadMore() {
    const cursor = nextBefore ?? list.data?.next_before;
    if (!cursor) return;
    setLoadingMore(true);
    try {
      const page = await client.listAuthzDecisions({ ...filters, before: cursor });
      setExtra((rows) => [...rows, ...page.items]);
      setNextBefore(page.next_before);
    } finally {
      setLoadingMore(false);
    }
  }

  const allRows: DecisionView[] = [...(list.data?.items ?? []), ...extra];
  const hasMore = (nextBefore ?? list.data?.next_before) != null;

  return (
    <section className="grid gap-6">
      <header>
        <h2 className="text-xl font-semibold tracking-tight">{m.decisions.title}</h2>
        <p className="text-sm text-[color:var(--color-subtle,#6b7280)]">{m.decisions.description}</p>
      </header>

      <Card>
        <CardHeader>
          <CardTitle>{m.decisions.filters.apply}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-4 sm:items-end">
            <div className="grid gap-1">
              <Label htmlFor="d-t">{m.decisions.filters.tenantLabel}</Label>
              <Input id="d-t" value={tenant} onChange={(e) => setTenant(e.currentTarget.value)} />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="d-s">{m.decisions.filters.subjectLabel}</Label>
              <Input id="d-s" value={subject} onChange={(e) => setSubject(e.currentTarget.value)} />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="d-e">{m.decisions.filters.effectLabel}</Label>
              <Select value={effect || "__any__"} onValueChange={(v) => setEffect(v === "__any__" ? "" : (v as RuleEffect))}>
                <SelectTrigger id="d-e"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="__any__">{m.common.any}</SelectItem>
                  <SelectItem value="allow">{m.common.allow}</SelectItem>
                  <SelectItem value="deny">{m.common.deny}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex gap-2">
              <Button onClick={onApply} className="flex-1">{m.decisions.filters.apply}</Button>
              <Button variant="outline" onClick={onReset}>{m.decisions.filters.reset}</Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {list.isLoading ? (
        <StateRow variant="loading">{m.common.loading}</StateRow>
      ) : list.error ? (
        <StateRow variant="error">{list.error.message || m.common.error}</StateRow>
      ) : allRows.length === 0 ? (
        <StateRow variant="empty">{m.common.empty}</StateRow>
      ) : (
        <>
          <DataTable
            label={m.decisions.title}
            headers={[
              m.common.at,
              m.common.tenant,
              m.common.subject,
              m.common.action,
              m.common.resource,
              m.common.effect,
              m.common.reason,
            ]}
            rows={allRows.map((d, i) => (
              <tr key={`${d.at}:${d.subject}:${i}`}>
                <Td className="whitespace-nowrap font-mono text-xs">{d.at}</Td>
                <Td>{d.tenant ?? "—"}</Td>
                <Td><code className="text-xs">{d.subject}</code></Td>
                <Td><code className="text-xs">{d.action}</code></Td>
                <Td>
                  <code className="text-xs">
                    {d.kind}
                    {d.id ? `:${d.id}` : ""}
                  </code>
                </Td>
                <Td>
                  <span
                    className={
                      d.effect === "allow"
                        ? "rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-700"
                        : "rounded-full bg-rose-100 px-2 py-0.5 text-xs font-medium text-rose-700"
                    }
                  >
                    {d.effect === "allow" ? m.common.allow : m.common.deny}
                  </span>
                </Td>
                <Td className="text-xs text-[color:var(--color-subtle,#6b7280)]">
                  {d.reason ?? d.rule_id ?? "—"}
                </Td>
              </tr>
            ))}
          />
          <div className="flex justify-center">
            {hasMore ? (
              <Button variant="outline" onClick={onLoadMore} disabled={loadingMore}>
                {loadingMore ? m.common.loading : m.decisions.loadMore}
              </Button>
            ) : (
              <p className="text-xs text-[color:var(--color-subtle,#6b7280)]">{m.decisions.endOfList}</p>
            )}
          </div>
        </>
      )}
    </section>
  );
}
