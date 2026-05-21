# @nube/starter-ui-skills

Reusable React components and hooks for managing a
[`starter-skills`](../../DOCS/agent/SKILLS.md) registry from a frontend
surface: list bundles, inspect their `SKILL.md` body and resources,
and approve / revoke quarantined ones.

- **Headless transport** — a `SkillsAdapter` interface; bring REST,
  Tauri commands, GraphQL, or a mock. An in-memory helper
  (`createInMemorySkillsAdapter`) ships for demos and tests.
- **Composable** — drop-in `<SkillsManager />` for the easy path, or
  compose `SkillList` + `SkillFilterBar` + `SkillDetail` for full
  control.
- **Zero I/O in the library** — no fetches, no stores, no globals.
  Same rule as `starter-ui-kit` and `starter-ui-chat`.
- **Tailwind v4 / shadcn tokens** — assumes the consumer loaded
  `@nube/starter-ui-kit/styles.css`.

## Install

```bash
pnpm add @nube/starter-ui-skills
```

Peer deps: `react`, `react-dom`, `@nube/starter-ui-kit`.

## Quick start

```tsx
import {
  SkillsManager,
  createInMemorySkillsAdapter,
  type Skill,
} from "@nube/starter-ui-skills";
import "@nube/starter-ui-kit/styles.css";

const fixture: Skill[] = [
  {
    id: "starter.ai-builder.dashboards",
    description: "Drafts, edits, and publishes ai-builder dashboards.",
    trust: "approved",
    bundleHash: "9f1e8b2c5a7d3e0f4b6c8a1d2e3f4b5c6d7e8f90a1b2c3d4e5f6a7b8c9d0e1f2",
    allowedTools: ["starter.mcp.call", "starter.flow.transform"],
    source: "host",
    body: "# Dashboards skill\n\n…verbatim SKILL.md body…",
    resources: [
      { uri: "file://prompt.md", contentHash: "abc…", name: "prompt.md" },
      { uri: "file://schema.json", contentHash: "def…", name: "schema.json" },
    ],
  },
];

export function Page() {
  const adapter = React.useMemo(
    () => createInMemorySkillsAdapter({ skills: fixture }),
    [],
  );
  return (
    <div className="h-screen">
      <SkillsManager adapter={adapter} />
    </div>
  );
}
```

## Wiring a real backend

There is no `createHttpSkillsAdapter` yet — the
[`starter-skills`](../../DOCS/agent/SKILLS.md) Rust crate does not
yet expose an HTTP surface (no handlers in `starter-server`, no
entries in `openapi.json`, nothing in `starter-client-ts`). The
shape it will take is tracked in
[SKILLS.md → S-D1](../../DOCS/agent/SKILLS.md#s-d1--approval-surfaces-cli--http--ui);
once those routes land we will ship the adapter here. Until then,
implement `SkillsAdapter` against whatever transport your host
uses — the interface is the same shape the HTTP layer will land on:

```ts
import type { SkillsAdapter } from "@nube/starter-ui-skills";

export const myAdapter: SkillsAdapter = {
  async list(signal) {
    const r = await fetch("/api/v1/skills", { signal });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    return r.json();
  },
  async get(id, signal) {
    const r = await fetch(`/api/v1/skills/${encodeURIComponent(id)}`, { signal });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    return r.json();
  },
  async approve(id, bundleHash, signal) {
    const r = await fetch(`/api/v1/skills/${encodeURIComponent(id)}/approve`, {
      method: "POST",
      signal,
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ bundleHash }),
    });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    return r.json();
  },
  async revoke(id, bundleHash, signal) {
    const r = await fetch(`/api/v1/skills/${encodeURIComponent(id)}/approve`, {
      method: "DELETE",
      signal,
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ bundleHash }),
    });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
  },
};
```

## Composing primitives

```tsx
import {
  SkillList,
  SkillFilterBar,
  SkillDetail,
  SkillActionButton,
  useSkills,
  useSkill,
} from "@nube/starter-ui-skills";

function MySkills({ adapter }: { adapter: SkillsAdapter }) {
  const { visible, filter, setFilter, search, setSearch, approve, revoke } =
    useSkills({ adapter });
  const [id, setId] = React.useState<string | null>(null);
  const { skill } = useSkill({ adapter, id });
  return (
    <div className="grid h-full grid-cols-[20rem_1fr] gap-4">
      <div className="flex flex-col gap-3">
        <SkillFilterBar
          filter={filter}
          onFilterChange={setFilter}
          search={search}
          onSearchChange={setSearch}
        />
        <SkillList skills={visible} selectedId={id} onSelect={(s) => setId(s.id)} />
      </div>
      {skill ? (
        <SkillDetail
          skill={skill}
          actions={
            skill.trust === "approved" ? (
              <SkillActionButton
                variant="destructive"
                onClick={() => revoke(skill.id, skill.bundleHash)}
              >
                Revoke
              </SkillActionButton>
            ) : (
              <SkillActionButton
                variant="primary"
                onClick={() => approve(skill.id, skill.bundleHash)}
              >
                Approve
              </SkillActionButton>
            )
          }
        />
      ) : null}
    </div>
  );
}
```

## Scope

- ✅ Components, hooks, types, in-memory adapter.
- ❌ No HTTP adapter shipped yet — backend route doesn't exist; see
  [SKILLS.md → S-D1](../../DOCS/agent/SKILLS.md#s-d1--approval-surfaces-cli--http--ui).
- ❌ No global state, no React context required.
- ❌ No markdown renderer bundled (zero-dep peer policy, same as
  `@nube/starter-ui-chat`'s `renderMessage`). `Skill.body` is the
  verbatim `SKILL.md` text — pass `renderBody` to `<SkillDetail>`
  or `<SkillsManager>` to plug your own renderer
  (`react-markdown`, `marked`, MDX, …).

## Behaviour notes

- **Auto-poll.** `<SkillsManager>` polls `adapter.list()` every
  `10_000` ms by default. Override with `refreshIntervalMs={n}`;
  pass `refreshIntervalMs={0}` to disable polling entirely (the
  Reload button still works). The lower-level `useSkills` hook
  does not poll by default — pass `refreshIntervalMs` to opt in.
- **Optimistic refresh.** `approve()` / `revoke()` re-fetch the
  list (and the open detail) on success so the trust badge and
  approval metadata update immediately.
- **Abortable.** Every adapter call receives an `AbortSignal`; the
  hooks abort the previous in-flight request on re-fetch and on
  unmount. Honour it in custom adapters.
