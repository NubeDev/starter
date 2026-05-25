// Explorer panel for the warehouse admin shell. Mounts
// `<Explorer/>` from `@nube/starter-ui-ch-explorer` (the headless
// library shared with the demo binary) and feeds it rubix i18n via
// the `i18n` prop.
//
// The rubix-specific overlays (FreshnessTiles, MartTree) live
// alongside the equivalent rubix-native panels rather than inside
// the Explorer tab: FreshnessTiles renders in `WarehouseInsightsPanel`
// (the operator surface for W11/W16 observability), and mart
// list/create/drop is covered by `WarehouseMartsPanel`. Keeping the
// Explorer tab pure sql-studio matches the original UX of the
// upstream fork.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import { useIntl } from 'react-intl'
import {
  Explorer,
  type ExplorerMessages,
} from '@nube/starter-ui-ch-explorer'

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

  return <Explorer i18n={messages} header={<></>} />
}

