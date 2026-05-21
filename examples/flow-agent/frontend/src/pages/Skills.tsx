// `/skills` — drop-in `<SkillsManager>` seeded with the two reference
// skill bundles already shipping in the monorepo
// (`starter.ai-builder.dashboards`, `starter.ai-builder.themes`) via
// `createInMemorySkillsAdapter`. No real backend is contacted.
//
// One bundle starts `approved`, the other `quarantined`, so the
// acceptance flow ("approving the quarantined one moves it to
// Approved without a refresh") is exercisable end-to-end.

import { useMemo } from "react"

import {
  SkillsManager,
  createInMemorySkillsAdapter,
  type Skill,
} from "@nube/starter-ui-skills"

// Verbatim body excerpts from the reference bundles under
// `/skills/starter.ai-builder.*/SKILL.md`. The frontmatter is hoisted
// into the structured Skill fields below; the body is the markdown
// after the `---` fence.

const DASHBOARDS_BODY = `# starter.ai-builder.dashboards

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
\`starter.ai-builder.themes\` instead.

## Operating contract

1. Open the current page draft with the \`page.open\` MCP tool.
   The draft id is in the input slot \`page_id\`; if absent,
   create a new draft with \`page.create\`.
2. Mutate the draft incrementally — one tool call per panel
   add, move, resize, or bind. Never replace the whole page in
   a single call; the editor transport expects diffs.
3. Validate the draft against \`schema.json\` (loaded as a
   resource) before publishing.
4. Publish only on explicit user confirmation.
`

const THEMES_BODY = `# starter.ai-builder.themes

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
\`starter.ai-builder.dashboards\` instead.

## Operating contract

1. Open the current theme draft with \`theme.open\`. The draft
   id is in the input slot \`theme_id\`; if absent, fork the
   default theme with \`theme.fork\`.
2. Edit tokens before component styles.
3. Use \`tokens.json\` (loaded as a resource) as the canonical
   list of token names — refuse to invent tokens.
4. Preview through \`theme.preview\` before publishing.
`

// The bundleHash is normally blake3 of the bundle bytes; for the
// fixture we use a stable hex placeholder that matches the shape so
// the approval round-trip works (the in-memory adapter just checks
// for equality on approve()).
const FIXTURE_SKILLS: Skill[] = [
  {
    id: "starter.ai-builder.dashboards",
    description:
      "Page-builder over MCP. Drafts, edits, and publishes ai-builder dashboards (pages, panels, layout grids, widget bindings) by driving the editor transport through the MCP tool surface.",
    trust: "approved",
    bundleHash:
      "9f1e8b2c5a7d3e0f4b6c8a1d2e3f4b5c6d7e8f90a1b2c3d4e5f6a7b8c9d0e1f2",
    allowedTools: ["starter.mcp.call", "starter.flow.transform"],
    modelHint: "claude-3-5-sonnet-latest",
    source: "host",
    approvedAt: new Date(Date.now() - 86_400_000).toISOString(),
    approvedBy: "operator",
    body: DASHBOARDS_BODY,
    resources: [
      {
        uri: "file://prompt.md",
        contentHash:
          "1a2b3c4d5e6f70819273645566778899aabbccddeeff00112233445566778899",
        name: "prompt.md",
        sizeBytes: 1820,
      },
      {
        uri: "file://schema.json",
        contentHash:
          "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        name: "schema.json",
        sizeBytes: 3240,
      },
    ],
  },
  {
    id: "starter.ai-builder.themes",
    description:
      "Theme-builder for ai-builder. Reuses the editor transport to edit theme tokens (colour, typography, spacing, radius, shadow) and component styles.",
    trust: "quarantined",
    quarantineReason: "no-approval-row",
    bundleHash:
      "7c3d2f1a8b4e5c6d9f0a1b2c3d4e5f607182930a4b5c6d7e8f90a1b2c3d4e5f6",
    allowedTools: ["starter.mcp.call", "starter.flow.transform"],
    modelHint: "claude-3-5-sonnet-latest",
    source: "host",
    body: THEMES_BODY,
    resources: [
      {
        uri: "file://prompt.md",
        contentHash:
          "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
        name: "prompt.md",
        sizeBytes: 1640,
      },
      {
        uri: "file://tokens.json",
        contentHash:
          "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
        name: "tokens.json",
        sizeBytes: 2120,
      },
    ],
  },
]

export function Skills() {
  // Memoise so the adapter (and its in-process mutable state) survives
  // re-renders — approving a skill must persist across the re-render
  // that the approval itself triggers.
  const adapter = useMemo(
    () => createInMemorySkillsAdapter({ skills: FIXTURE_SKILLS }),
    [],
  )
  return (
    <div className="flex h-[calc(100dvh-3.5rem)] min-h-0 w-full flex-col">
      <SkillsManager adapter={adapter} />
    </div>
  )
}
