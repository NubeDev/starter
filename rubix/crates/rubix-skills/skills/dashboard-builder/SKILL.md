---
id: com.rubix.dashboard-builder
description: |
  Build, edit, and list rubix dashboards via the SDUI surface. Pick
  this skill when the user asks for a new page, a chart, a widget
  layout, or to update an existing dashboard.
allowed_tools:
  - rubix.dashboard.create
  - rubix.dashboard.update
  - rubix.dashboard.list
  - rubix.dashboard.page_set
trust: approved
---

# Dashboard builder

You compose SDUI pages from rubix's primitive widgets. You do not
write JSX, HTML, or CSS — you produce SDUI page definitions that
the frontend renders.

## How to work

1. Ask what data the page should show before picking a layout. A
   "show me disk usage" request becomes one chart, not a four-panel
   dashboard.
2. Use `rubix.dashboard.list` to check whether a similar page
   already exists. Reusing an existing dashboard is almost always
   better than spawning a near-duplicate.
3. When updating an existing page, prefer `dashboard.page_set` with
   the changed widgets only; do not regenerate the whole page from
   scratch — that loses the operator's manual edits.

## What not to do

- Do not invent widget types. Stick to what the SDUI catalogue
  exposes through the descriptor.
- Do not write tenant-scoped data references without confirming
  the dashboard's tenant context.
- Do not produce "placeholder" widgets ("Coming soon", "TBD") —
  either build the widget or say you can't.
