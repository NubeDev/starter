//! Seed the bundled rubix flow YAMLs into the
//! `flows_definitions` Postgres table on first boot, then load
//! every live revision back so the `FlowRegistry` sees PG as the
//! source of truth.
//!
//! Phase D.1 contract:
//!
//! - **Idempotent seed.** For each bundled YAML, INSERT a row
//!   under the all-zero `(tenant_id, created_by)` sentinel only
//!   when no live row exists for `(tenant_id, flow_id)`. The
//!   `ON CONFLICT DO NOTHING` clause on the
//!   `(tenant_id, flow_id, revision_id)` UNIQUE plus the
//!   live-check together make a second boot a no-op (the
//!   second-boot assertion in
//!   `tests/flow_definitions_seed_test.rs`).
//! - **Load from PG.** After seeding, `SELECT flow_id,
//!   revision_id, body_yaml FROM flows_definitions WHERE
//!   tenant_id = $0 AND superseded_at IS NULL`. Each `body_yaml`
//!   is re-parsed through `rubix_flows::parse_yaml` +
//!   `rubix_flows::convert`; the freshly-minted revision the
//!   converter produces is swapped for the row's `revision_id`
//!   so the in-memory id matches the PG row exactly.
//! - **Laptop fallback.** When the pool is `None` (no
//!   `RUBIX_DATABASE_URL`) the seeder no-ops and the caller falls
//!   back to `rubix_flows::load_all()` directly so the binary
//!   still boots without Postgres.
//!
//! See `docs/design/flows/` for the seed-load contract and
//! `rubix-store-postgres/migrations/flows_definitions/` for the
//! table shape.

use anyhow::Result;
use starter_flow::definition::body::FlowBody;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_store_postgres::pool::Pool;
use tracing::{debug, info};
use uuid::Uuid;

/// The all-zero UUID used as both `tenant_id` and `created_by`
/// for bundled flow rows. Mirrors the same sentinel the
/// `undo_snapshots` table uses for system-scope writes.
pub const SYSTEM_TENANT: Uuid = Uuid::nil();

/// Walk [`rubix_flows::BUNDLED`], INSERT any missing rows into
/// `flows_definitions`, then SELECT every live revision and
/// convert each back to the triple shape the `FlowRegistry`
/// expects. Returns the count of rows inserted by this call so
/// the caller can log first-boot vs steady-state.
pub async fn seed_and_load(
    pool: &Pool,
) -> Result<(Vec<(FlowId, FlowRevisionId, FlowBody)>, usize)> {
    let mut inserted = 0usize;
    for (path, bytes) in bundled_pairs() {
        let yaml = rubix_flows::parse_yaml(&path, &bytes)
            .map_err(|e| anyhow::anyhow!("parse bundled yaml `{path}`: {e}"))?;
        let flow_id = yaml.id.clone();

        // Probe live-row existence per (tenant, flow_id) — the
        // miss path inserts a new revision, the hit path leaves
        // the row alone so the seed is idempotent across boots.
        // PostgreSQL types the integer literal `1` as INT4, so
        // decode into i32 (not i64) or sqlx will refuse on hit.
        let exists: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM flows_definitions
              WHERE tenant_id = $1::uuid
                AND flow_id = $2
                AND superseded_at IS NULL
              LIMIT 1",
        )
        .bind(SYSTEM_TENANT)
        .bind(&flow_id)
        .fetch_optional(pool.sqlx())
        .await?;
        if exists.is_some() {
            debug!(flow_id = %flow_id, "flows_definitions seed: skipped (live row present)");
            continue;
        }

        let revision = FlowRevisionId::new();
        let id = ulid_text();
        let body_yaml = std::str::from_utf8(&bytes)
            .map_err(|e| anyhow::anyhow!("bundled yaml `{path}` not utf8: {e}"))?;
        let rows = sqlx::query(
            "INSERT INTO flows_definitions
                (id, tenant_id, flow_id, revision_id, body_yaml, created_by)
             VALUES ($1, $2::uuid, $3, $4, $5, $6::uuid)
             ON CONFLICT (tenant_id, flow_id, revision_id) DO NOTHING",
        )
        .bind(&id)
        .bind(SYSTEM_TENANT)
        .bind(&flow_id)
        .bind(revision.to_string())
        .bind(body_yaml)
        .bind(SYSTEM_TENANT)
        .execute(pool.sqlx())
        .await?;
        inserted += rows.rows_affected() as usize;
        debug!(flow_id = %flow_id, revision = %revision, "flows_definitions seed: inserted");
    }

    // Load every live revision back. Order by `created_at` so
    // the boot log lists flows in the order they were first
    // seeded (deterministic in steady state).
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT flow_id, revision_id, body_yaml
           FROM flows_definitions
          WHERE tenant_id = $1::uuid AND superseded_at IS NULL
          ORDER BY created_at ASC",
    )
    .bind(SYSTEM_TENANT)
    .fetch_all(pool.sqlx())
    .await?;

    let mut triples = Vec::with_capacity(rows.len());
    for (flow_id_text, revision_text, body_yaml) in rows {
        let path = format!("pg://flows_definitions/{flow_id_text}");
        let yaml = rubix_flows::parse_yaml(&path, body_yaml.as_bytes())
            .map_err(|e| anyhow::anyhow!("parse pg-loaded yaml `{path}`: {e}"))?;
        let (flow_id, _fresh_rev, body) = rubix_flows::convert(&path, yaml)
            .map_err(|e| anyhow::anyhow!("convert pg-loaded yaml `{path}`: {e}"))?;
        let revision: FlowRevisionId = revision_text
            .parse::<Uuid>()
            .map(FlowRevisionId)
            .map_err(|e| anyhow::anyhow!("revision_id `{revision_text}` not a uuid: {e}"))?;
        // Sanity-check the round-trip — the PG flow_id text must
        // equal the parsed flow id; otherwise an operator edited
        // a row's `flow_id` column out from under the body.
        if flow_id.to_string() != flow_id_text {
            anyhow::bail!(
                "flows_definitions row mismatch: flow_id column = `{flow_id_text}`, body declares `{flow_id}`",
            );
        }
        triples.push((flow_id, revision, body));
    }

    info!(
        inserted,
        loaded = triples.len(),
        "flows_definitions seed-and-load complete"
    );
    Ok((triples, inserted))
}

/// Collect every bundled `*.yaml` / `*.yml` file as
/// `(relative-path, bytes)` so the seeder can store the body
/// verbatim and re-parse on load.
fn bundled_pairs() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    collect(&rubix_flows::BUNDLED, &mut out);
    out
}

fn collect(dir: &include_dir::Dir<'_>, out: &mut Vec<(String, Vec<u8>)>) {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::File(f) => {
                let path = f.path().to_string_lossy().into_owned();
                if path.ends_with(".yaml") || path.ends_with(".yml") {
                    out.push((path, f.contents().to_vec()));
                }
            }
            include_dir::DirEntry::Dir(sub) => collect(sub, out),
        }
    }
}

/// Crockford-base32-ish ULID surrogate built from a UUIDv4. The
/// table column is TEXT and order-sensitive paths use
/// `created_at`, so a unique 26-char string is enough for the
/// PRIMARY KEY constraint — full ULID monotonicity is not
/// required for this seed path.
fn ulid_text() -> String {
    // 26 chars matches the ULID width referenced in the
    // `undo_snapshots` migration so any future tooling that
    // parses either table's `id` column sees a uniform width.
    let raw = Uuid::new_v4().simple().to_string();
    raw.chars().take(26).collect()
}
