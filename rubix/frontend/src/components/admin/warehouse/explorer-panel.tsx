// Explorer panel for the warehouse admin shell. Mounts
// `<Explorer/>` from `@nube/starter-ui-ch-explorer` (the headless
// library shared with the demo binary) and feeds it rubix i18n via
// the `i18n` prop. The rubix-specific overlays (W11/W16 freshness
// tiles + materialised marts) come from the library's `./rubix`
// sub-export, which uses `@nube/rubix-client-react` typed hooks for
// every destructive call.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import { useIntl } from 'react-intl'
import {
  Explorer,
  type ExplorerMessages,
} from '@nube/starter-ui-ch-explorer'
import {
  FreshnessTiles,
  MartTree,
} from '@nube/starter-ui-ch-explorer/rubix'

export function WarehouseExplorerPanel() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  // Map rubix message ids onto the explorer's i18n shape. Only the
  // labels the operator actually sees are overridden; everything
  // else falls back to `DEFAULT_EXPLORER_MESSAGES`.
  const messages: Partial<ExplorerMessages> = {
    shell: {
      title: tr(
        'admin.warehouse.explorer.title',
        'ClickHouse explorer',
      ),
      tabs: {
        overview: tr(
          'admin.warehouse.explorer.tabs.overview',
          'Overview',
        ),
        tables: tr('admin.warehouse.explorer.tabs.tables', 'Tables'),
        schema: tr('admin.warehouse.explorer.tabs.schema', 'Schema'),
        query: tr('admin.warehouse.explorer.tabs.query', 'Query'),
      },
    },
  }

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
      <MartTree
        messages={{
          title: tr('admin.warehouse.explorer.marts.title', 'Rubix marts'),
          emptyTitle: tr(
            'admin.warehouse.explorer.marts.emptyTitle',
            'No marts',
          ),
          emptyDescription: tr(
            'admin.warehouse.explorer.marts.emptyDescription',
            'Create a mart with the rubix.clickhouse.mart.create verb to materialise an L1–L3 aggregate.',
          ),
          loadError: tr(
            'admin.warehouse.explorer.marts.loadError',
            'Failed to load marts.',
          ),
          drop: tr('admin.warehouse.explorer.marts.drop', 'Drop'),
          confirmTitle: tr(
            'admin.warehouse.explorer.marts.confirmTitle',
            'Drop mart "{name}"?',
          ),
          confirmDescription: tr(
            'admin.warehouse.explorer.marts.confirmDescription',
            'This deletes the underlying table and all its data. The operation is reversible via rubix.undo.last, but only until the next mutating call is recorded.',
          ),
          confirmAction: tr(
            'admin.warehouse.explorer.marts.confirmAction',
            'Drop mart',
          ),
          confirmCancel: tr(
            'admin.warehouse.common.cancel',
            'Cancel',
          ),
        }}
      />
      <Explorer i18n={messages} header={<></>} />
    </div>
  )
}
