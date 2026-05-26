// Insights rules panel. Lists `rubix.insights.rule.*` rules with
// enable/disable toggles and a create form (id + body YAML).
//
// Also surfaces the W11/W16 freshness tiles at the top of the
// panel. Freshness lives here because Insights is the rubix
// operator surface for warehouse observability; the Explorer tab
// stays pure sql-studio per design/warehouse/explorer/README.md.

import { useState } from 'react'
import { useIntl } from 'react-intl'
import {
  Button,
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  Input,
  Label,
  Skeleton,
  Switch,
  Textarea,
} from '@nube/starter-ui-kit'
import { Activity, Plus } from 'lucide-react'
import {
  useInsightsRuleCreate,
  useInsightsRuleDisable,
  useInsightsRuleEnable,
  useInsightsRulesList,
} from '@nube/rubix-client-react'
import { FreshnessTiles } from '@nube/starter-ui-warehouse-explorer/rubix'

export function WarehouseInsightsPanel() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const list = useInsightsRulesList()
  const create = useInsightsRuleCreate()
  const enable = useInsightsRuleEnable()
  const disable = useInsightsRuleDisable()

  const [ruleId, setRuleId] = useState('')
  const [bodyYaml, setBodyYaml] = useState('')

  async function submit() {
    if (!ruleId.trim() || !bodyYaml.trim()) return
    await create.mutateAsync({ rule_id: ruleId.trim(), body_yaml: bodyYaml })
    setRuleId('')
    setBodyYaml('')
  }

  async function toggle(id: string, currentlyEnabled: boolean) {
    if (currentlyEnabled) {
      await disable.mutateAsync({ rule_id: id })
    } else {
      await enable.mutateAsync({ rule_id: id })
    }
  }

  const rows = list.data?.rules ?? []

  return (
    <div className="space-y-6">
      <FreshnessTiles
        messages={{
          entitiesTitle: tr(
            'admin.warehouse.explorer.freshness.entities',
            'Entities dictionary',
          ),
          ingestLagTitle: tr(
            'admin.warehouse.explorer.freshness.ingestLag',
            'Ingest lag (W16)',
          ),
          ingestBacklogTitle: tr(
            'admin.warehouse.explorer.freshness.ingestBacklog',
            'Ingest backlog',
          ),
          awaitingFirstRefresh: tr(
            'admin.warehouse.explorer.freshness.awaiting',
            'Awaiting first refresh.',
          ),
          ingestLagSubtitle: tr(
            'admin.warehouse.explorer.freshness.ingestLagSubtitle',
            'Oldest async-insert part awaiting flush.',
          ),
          ingestBacklogSubtitle: tr(
            'admin.warehouse.explorer.freshness.ingestBacklogSubtitle',
            'Bytes pending in system.asynchronous_inserts.',
          ),
        }}
      />
      <div className="rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)] p-4">
        <div className="mb-3 flex items-center gap-2 text-sm font-medium">
          <Plus className="h-4 w-4" />
          {tr('admin.warehouse.insights.createTitle', 'New insights rule')}
        </div>
        <div className="grid gap-3 sm:grid-cols-[200px_1fr]">
          <div>
            <Label>{tr('admin.warehouse.insights.idLabel', 'Rule id')}</Label>
            <Input
              value={ruleId}
              onChange={(e) => setRuleId(e.target.value)}
              placeholder="boiler-runtime-anomaly"
            />
          </div>
          <div>
            <Label>{tr('admin.warehouse.insights.bodyLabel', 'Body (YAML)')}</Label>
            <Textarea
              className="font-mono text-xs"
              rows={6}
              value={bodyYaml}
              onChange={(e) => setBodyYaml(e.target.value)}
              placeholder={'window: 1h\nmetric: ...'}
            />
          </div>
        </div>
        <div className="mt-3 flex justify-end">
          <Button onClick={submit} disabled={create.isPending} size="sm">
            {tr('admin.warehouse.common.create', 'Create')}
          </Button>
        </div>
      </div>

      {list.isLoading ? (
        <div className="space-y-3">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : rows.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <Activity />
            </EmptyMedia>
            <EmptyTitle>
              {tr('admin.warehouse.insights.empty.title', 'No insights rules')}
            </EmptyTitle>
            <EmptyDescription>
              {tr(
                'admin.warehouse.insights.empty.body',
                'Add a rule above to detect anomalies in the warehouse.',
              )}
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <div className="space-y-2">
          {rows.map((r) => (
            <div
              key={r.rule_id}
              className="flex items-center justify-between gap-3 rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)] px-4 py-3"
            >
              <div className="min-w-0">
                <div className="font-mono text-sm font-medium">{r.rule_id}</div>
                {r.name ? (
                  <div className="text-xs text-[color:var(--color-muted)]">
                    {r.name}
                  </div>
                ) : null}
              </div>
              <div className="flex items-center gap-3">
                <span className="text-xs text-[color:var(--color-muted)]">
                  {r.enabled
                    ? tr('admin.warehouse.insights.enabled', 'Enabled')
                    : tr('admin.warehouse.insights.disabled', 'Disabled')}
                </span>
                <Switch
                  checked={r.enabled}
                  onCheckedChange={() => toggle(r.rule_id, r.enabled)}
                />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
