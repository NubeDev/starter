# 01 — Dashboard storage

> **Tier:** scope (plan). Lifetime: weeks. Not referenced from code.
> See [README.md](./README.md) for the full plan.

## What this file decides

Where dashboard page bodies live, how revisions are tracked, and
how page authz is wired. Mirrors the pattern PR #32 used for
`flows_definitions` so reviewers can read one and infer the other.

## The decision (resolves SDUI design Q3)

**Option B — rubix-owned Postgres table.** Reasons:

1. `starter-sdui-routes` exposes a `PageProvider` *trait*, not a
   write API. Implementing it is the contract; building one is not.
2. Rubix already owns the migration story for flow definitions in
   `rubix-store-postgres`; mirroring that pattern keeps the
   reviewer's mental model small.
3. Rubix-side authz (next section) needs to register every page
   as a `ResourceSpec` at write time, which is easier when rubix
   owns the write path.

Upstream option A (a write path inside `starter-sdui-routes`) can
land later if a second consumer wants it. The trait surface stays
unchanged either way.

## The table

`rubix-store-postgres/migrations/NNNN_dashboards.sql`:

```sql
CREATE TABLE dashboards_definitions (
  page_id          TEXT NOT NULL,
  revision_id      UUID NOT NULL DEFAULT gen_random_uuid(),
  body_json        JSONB NOT NULL,
  tenant_id        TEXT NOT NULL,
  owner_principal  TEXT NOT NULL,
  title            TEXT NOT NULL,
  tags             TEXT[] NOT NULL DEFAULT '{}',
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  created_by       TEXT NOT NULL,
  superseded_at    TIMESTAMPTZ NULL,
  PRIMARY KEY (page_id, revision_id)
);

CREATE INDEX dashboards_definitions_active_idx
  ON dashboards_definitions (tenant_id, page_id)
  WHERE superseded_at IS NULL;

CREATE INDEX dashboards_definitions_tags_idx
  ON dashboards_definitions USING GIN (tags)
  WHERE superseded_at IS NULL;
```

Rules:

- `page_id` is the SDUI page id (e.g. `"dashboard.disk-overview"`).
  Stable across revisions.
- Every write is **insert-only**. Updates supersede by setting
  `superseded_at = now()` on the previous active row and
  inserting a new one.
- "Latest" = `superseded_at IS NULL` (there should be exactly one
  per `page_id`; a partial unique index would be redundant but
  the writer enforces it).
- `body_json` is a fully-typed `starter_ui_ir::ComponentTree` with
  `ir_version` stamped. Validation runs at write time.
- `tags[]` powers "show me dashboards tagged X" via the
  `starter-tags` substrate.

## NOTIFY / hot-reload

Mirror `rubix_flows_definitions`:

```sql
CREATE OR REPLACE FUNCTION dashboards_notify() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify('rubix_dashboards_definitions',
    json_build_object('page_id', NEW.page_id, 'revision_id', NEW.revision_id)::text);
  RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE TRIGGER dashboards_notify_ins
  AFTER INSERT ON dashboards_definitions
  FOR EACH ROW EXECUTE FUNCTION dashboards_notify();
```

`rubix-agent/src/boot/dashboards_notify.rs` listens on the channel
and invalidates the in-process `PageProvider` cache (next file).
Same shape as `boot/flow_notify.rs`.

## Bundled vs operator vs AI-authored pages

Three sources, one table:

| Source | `created_by` | `owner_principal` | Mutable by operator? |
|---|---|---|---|
| Bundled (seeded at boot from `include_dir!`) | `"system"` | `"system"` | No — refuse `dashboard.update` / `delete` |
| Operator-authored | `op@example.com` | `op@example.com` | Yes |
| AI-authored via tool call | the LLM's flow run principal | the caller of the parent flow | Yes (by caller) |

Bundled pages live in
[`rubix/crates/rubix-flows/dashboards/`](../../../crates/rubix-flows/)
(new folder) as `.json` files, embedded via `include_dir!`. On boot,
`boot/dashboards_seed.rs` upserts any rows with `created_by="system"`
that are missing — never overwrites operator changes (see
`docs/design/sdui/README.md` once promoted).

## Authz registration

Every active page row registers a `ResourceSpec` in
`starter-authz::ResourceRegistry`. Spec shape:

```rust
ResourceSpec {
    kind: "rubix.dashboard.page",
    id: format!("{tenant_id}:{page_id}"),
    actions: &["view", "edit", "delete"],
}
```

A user without `view` grant for a given page id sees **404**, not
403 (deny-leakage prevention — already in
[`docs/design/sdui/README.md`](../../design/sdui/README.md)).
Default policy (StaticRbacEngine v0):

- `view` — anyone in the same `tenant_id` as the page.
- `edit` / `delete` — `owner_principal` or anyone with
  `rubix.dashboard:admin`.

Bundled pages are world-`view` within the tenant; nobody has
`edit` / `delete`.

## Rust API the rest of the system uses

`rubix-store-postgres/src/dashboards.rs`:

```rust
pub struct PgDashboardStore { pool: PgPool }

impl PgDashboardStore {
    pub async fn get_active(&self, page_id: &str)
        -> Result<Option<DashboardRevision>, StoreError>;
    pub async fn insert_revision(&self, ins: NewRevision)
        -> Result<DashboardRevision, StoreError>;
    pub async fn supersede(&self, page_id: &str)
        -> Result<u64, StoreError>;
    pub async fn list_active(&self, tenant: &str, filter: &ListFilter)
        -> Result<Vec<DashboardRevision>, StoreError>;
    pub async fn history(&self, page_id: &str)
        -> Result<Vec<DashboardRevision>, StoreError>;
}
```

One verb per file (`get_active.rs`, `insert.rs`, `supersede.rs`,
`list.rs`, `history.rs`) under `src/dashboards/`. The struct's
`impl` blocks live alongside; the `PgDashboardStore` type
declaration lives in `mod.rs`.

## File layout

```
rubix/crates/rubix-store-postgres/
  src/
    dashboards/
      mod.rs              ← struct + barrel
      get_active.rs       ← SELECT … WHERE superseded_at IS NULL
      insert.rs           ← INSERT new revision + UPDATE supersede
      supersede.rs        ← only-supersede (used by delete)
      list.rs             ← list filtered by tenant + tags
      history.rs          ← list every revision for one page_id
      error.rs            ← optimistic-concurrency / not-found
  migrations/
    NNNN_dashboards.sql
    NNNN_dashboards_notify.sql
```

## Tests in the same diff

- `tests/dashboards_insert_revision_test.rs` — round-trip,
  supersedes prior, NOTIFY fires.
- `tests/dashboards_optimistic_concurrency_test.rs` —
  `expected_revision_id` mismatch returns the structured
  `Conflict` variant.
- `tests/dashboards_authz_404_not_403_test.rs` — denied principal
  gets 404 from the resolve path (covered with mock authz; full
  HTTP test lives in `04-tools.md`).

## Open questions deferred to [08-open-questions.md](./08-open-questions.md)

- Q1 — should the bundled-page upsert also delete operator-authored
  pages whose `page_id` collides? Default: no (operator wins).
- Q2 — does the AI builder write under its own principal or the
  flow-caller's? Default: caller's (so undo + audit work).

## Acceptance for this slice

1. Migrations apply on a fresh PG (idempotent re-run).
2. `PgDashboardStore::insert_revision("dash.test", ...)` creates a
   row, fires NOTIFY, and a second insert against the same
   `page_id` supersedes the first.
3. `list_active` returns only non-superseded rows scoped to the
   caller's tenant.
4. `history` returns every revision in `created_at DESC` order.
5. `PageProvider` impl (in [03-host-glue.md](./03-host-glue.md))
   reads from this store; one round-trip from the resolver to PG.
