---
id: starter.ai-builder.dashboards
description: >-
  Page-builder over MCP. Drafts, edits, and publishes ai-builder
  dashboards (pages, panels, layout grids, widget bindings) by
  driving the editor transport through the MCP tool surface.
  Use when the user asks to create a dashboard, lay out panels,
  bind a panel to a data source, or publish a draft page.
allowed_tools:
  - starter.mcp.call
  - starter.flow.transform
model_hint: claude-3-5-sonnet-latest
trust: approved
resources:
  - file://prompt.md
  - file://schema.json
---

# starter.ai-builder.dashboards

You build dashboards inside ai-builder by issuing tool calls
against the MCP editor transport. A dashboard is a tree of
panels arranged on a layout grid; each panel binds to a data
source and a visualisation kind (table, line, bar, stat,
markdown).

## When to use this skill

Pick this skill when the user's request mentions any of:

- "dashboard", "page", "panel", "widget", "chart", "layout"
- "publish", "preview", "draft" in the ai-builder context
- a concrete visualisation kind (line chart, bar chart, table,
  stat, markdown) attached to a data source

If the request is purely about colour, typography, spacing
tokens, or component themes, prefer
`starter.ai-builder.themes` instead.

## Operating contract

1. Open the current page draft with the `page.open` MCP tool.
   The draft id is in the input slot `page_id`; if absent,
   create a new draft with `page.create`.
2. Mutate the draft incrementally — one tool call per panel
   add, move, resize, or bind. Never replace the whole page in
   a single call; the editor transport expects diffs.
3. Validate the draft against `schema.json` (loaded as a
   resource) before publishing. The schema is the source of
   truth for which panel kinds and which binding shapes are
   allowed; reject anything the schema does not cover rather
   than guessing.
4. Publish only on explicit user confirmation. A draft that
   has not been confirmed stays in draft state — never
   auto-publish.

See `prompt.md` for the verbatim system prompt the model uses
inside the editor; the body you are reading is the operator-
facing description, not the runtime prompt.
