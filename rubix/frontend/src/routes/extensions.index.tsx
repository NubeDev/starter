// `/extensions` index — the installed-extension admin table.
//
// Lives at the exact path `/extensions`; the per-extension route view
// at `/extensions/$extId/$rest` (see `extensions.$extId.$.tsx`)
// renders inside the same `<Outlet />` parent (`extensions.tsx`).
//
// Splitting the index out of `extensions.tsx` makes the parent route
// a pure layout — without this, TanStack file-based routing would
// keep matching `/extensions` for any sub-path because the parent's
// component never rendered `<Outlet />`.

import { createFileRoute } from '@tanstack/react-router'

import { ExtensionsTable } from './extensions'

export const Route = createFileRoute('/extensions/')({
  component: ExtensionsTable,
})
