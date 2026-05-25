// Tabbed shell for the rubix-side warehouse admin surface. Mirrors
// the structure of `<AuthzAdmin>` from `@nube/starter-ui-authz`:
// one set of `<Tabs>` from `@nube/starter-ui-kit` with one
// `<TabsContent>` per panel. Each panel is a self-contained verb
// component under `./*.tsx`.

import { useState } from 'react'
import { useIntl } from 'react-intl'
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@nube/starter-ui-kit'
import { WarehouseRulesPanel } from './rules-panel'
import { WarehouseMartsPanel } from './marts-panel'
import { WarehouseRetentionPanel } from './retention-panel'
import { WarehouseInsightsPanel } from './insights-panel'
import { WarehouseExplorerPanel } from './explorer-panel'

export type WarehouseAdminTab =
  | 'rules'
  | 'marts'
  | 'retention'
  | 'insights'
  | 'explorer'

export interface WarehouseAdminProps {
  defaultTab?: WarehouseAdminTab
}

export function WarehouseAdmin({ defaultTab = 'rules' }: WarehouseAdminProps) {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })
  const [tab, setTab] = useState<WarehouseAdminTab>(defaultTab)

  return (
    <Tabs value={tab} onValueChange={(v) => setTab(v as WarehouseAdminTab)}>
      <TabsList className="flex-wrap">
        <TabsTrigger value="rules">
          {tr('admin.warehouse.tabs.rules', 'Rules')}
        </TabsTrigger>
        <TabsTrigger value="marts">
          {tr('admin.warehouse.tabs.marts', 'Marts')}
        </TabsTrigger>
        <TabsTrigger value="retention">
          {tr('admin.warehouse.tabs.retention', 'Retention')}
        </TabsTrigger>
        <TabsTrigger value="insights">
          {tr('admin.warehouse.tabs.insights', 'Insights')}
        </TabsTrigger>
        <TabsTrigger value="explorer">
          {tr('admin.warehouse.tabs.explorer', 'Explorer')}
        </TabsTrigger>
      </TabsList>
      <TabsContent value="rules" className="mt-6">
        <WarehouseRulesPanel />
      </TabsContent>
      <TabsContent value="marts" className="mt-6">
        <WarehouseMartsPanel />
      </TabsContent>
      <TabsContent value="retention" className="mt-6">
        <WarehouseRetentionPanel />
      </TabsContent>
      <TabsContent value="insights" className="mt-6">
        <WarehouseInsightsPanel />
      </TabsContent>
      <TabsContent value="explorer" className="mt-6">
        <WarehouseExplorerPanel />
      </TabsContent>
    </Tabs>
  )
}
