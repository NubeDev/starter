---
id: starter.ai-builder.themes
description: >-
  Theme-builder for ai-builder. Reuses the editor transport to
  edit theme tokens (colour, typography, spacing, radius, shadow)
  and component styles. Use when the user asks to restyle a
  page, change the palette, swap fonts, tweak spacing, or
  produce a light/dark variant of an existing theme.
allowed_tools:
  - starter.mcp.call
  - starter.flow.transform
model_hint: claude-3-5-sonnet-latest
trust: approved
resources:
  - file://prompt.md
  - file://tokens.json
---

# starter.ai-builder.themes

You edit ai-builder themes through the same MCP editor
transport the dashboards skill uses. A theme is a flat token
set (colour, typography, spacing, radius, shadow) plus a
component-style overlay that maps tokens onto specific UI
parts.

## When to use this skill

Pick this skill when the user's request mentions any of:

- "theme", "palette", "colour" / "color", "dark mode",
  "light mode"
- "typography", "font", "type scale"
- "spacing", "radius", "shadow", "elevation"
- "restyle", "rebrand", "house style"

If the request is about adding or arranging panels rather
than restyling them, prefer
`starter.ai-builder.dashboards` instead.

## Operating contract

1. Open the current theme draft with `theme.open`. The draft
   id is in the input slot `theme_id`; if absent, fork the
   default theme with `theme.fork`.
2. Edit tokens before component styles. A token change is one
   tool call; never edit the whole token set in a single
   replacement.
3. Use `tokens.json` (loaded as a resource) as the canonical
   list of token names — refuse to invent tokens. Adding a
   new token is a separate flow handled outside this skill.
4. Preview through `theme.preview` before publishing.
   Publish only on explicit user confirmation.

See `prompt.md` for the verbatim system prompt the model uses
inside the editor; the body you are reading is the operator-
facing description, not the runtime prompt.
