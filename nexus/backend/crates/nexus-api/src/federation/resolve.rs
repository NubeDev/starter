//! Resolve a request's `sources` (alias → datasource id) into the engine's
//! resolved `FederatedSource`s, authorising every referenced datasource against
//! the caller's tenant before any data is read.
//!
//! This is the tenancy boundary for a federated query (RW-05 scope §4): an alias
//! may only resolve to a datasource the caller's tenant owns and may `view`.
//! Resolution is RLS-scoped (`datasource::get` sees only the tenant's rows, so a
//! cross-tenant id reads as `NotFound`, never leaking existence) and additionally
//! checks the `view` grant, matching the single-datasource query gate. Creds are
//! recovered through the same audited envelope path the sink resolver uses; the
//! plaintext lives only inside the returned source and is handed straight to the
//! engine.

use nexus_engine::{FederatedSource, PostgresConn};
use nexus_store::datasource;
use starter_spi::auth::Principal;
use starter_spi::Error;
use uuid::Uuid;

use crate::authz::{self, ACTION_VIEW, KIND_DATASOURCE};
use crate::state::AppState;

/// Resolve each `(alias, datasource, table)` ref into an `(alias, source)` pair.
/// `actor` is recorded on each secret-decrypt audit. Fails with `NotFound` for a
/// datasource the tenant cannot see, `Forbidden` for one it may not `view`, and
/// `Invalid` for a malformed id, an unsupported kind, or a missing required field
/// — a federated query naming anything it may not read must fail loudly, never
/// silently drop the source.
pub async fn resolve_sources(
    state: &AppState,
    principal: &Principal,
    tenant: &str,
    actor: &str,
    refs: &[nexus_spi::dto::query::FederatedSourceRef],
) -> Result<Vec<(String, FederatedSource)>, Error> {
    let mut out = Vec::with_capacity(refs.len());
    for r in refs {
        validate_alias(&r.alias)?;
        let id = Uuid::parse_str(&r.datasource).map_err(|_| Error::Invalid {
            message: format!("federated source {:?} id is not a uuid", r.alias),
        })?;
        let source = resolve_one(state, principal, tenant, actor, id, r).await?;
        out.push((r.alias.clone(), source));
    }
    Ok(out)
}

/// Resolve one reference: fetch the tenant's record, check `view`, and build the
/// engine source for its kind.
async fn resolve_one(
    state: &AppState,
    principal: &Principal,
    tenant: &str,
    actor: &str,
    id: Uuid,
    r: &nexus_spi::dto::query::FederatedSourceRef,
) -> Result<FederatedSource, Error> {
    let record = datasource::get(&state.metadata, tenant, id)
        .await?
        .ok_or_else(|| Error::NotFound {
            what: format!("datasource {id}"),
        })?;
    if !authz::can(
        state.engine.as_ref(),
        principal,
        ACTION_VIEW,
        KIND_DATASOURCE,
        &id.to_string(),
        tenant,
    )
    .await
    {
        return Err(Error::Forbidden);
    }

    match record.kind.as_str() {
        "postgres" => {
            let table = r.table.as_deref().ok_or_else(|| Error::Invalid {
                message: format!("postgres federated source {:?} requires a table", r.alias),
            })?;
            let secret = datasource::open_secret(&state.metadata, &state.envelope, tenant, actor, id)
                .await?;
            let port = u16::try_from(record.port).map_err(|_| Error::Invalid {
                message: format!("datasource port {} out of range", record.port),
            })?;
            Ok(FederatedSource::Postgres {
                conn: PostgresConn {
                    host: record.host,
                    port,
                    database: record.database,
                    user: record.db_user,
                    password: secret,
                },
                table: table.to_string(),
            })
        }
        // File datasource kinds (parquet/csv) are declared in the kinds pack and
        // supported by the engine, but persisting one needs a datasource record
        // that carries a path and no secret — a store-side change tracked for an
        // RW-04 fix pass (see rewrite/sessions/TODOs.md). Until then a file
        // datasource cannot be stored, so resolving one is an explicit Invalid.
        other => Err(Error::Invalid {
            message: format!("datasource kind '{other}' is not yet usable as a federated source"),
        }),
    }
}

/// A federated alias becomes the SQL table `ds_<alias>`, so it must be a plain
/// identifier. Restricting it here keeps the engine's table registration safe
/// without the engine re-validating caller input.
fn validate_alias(alias: &str) -> Result<(), Error> {
    let mut chars = alias.chars();
    let first_ok = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_');
    if first_ok && chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(())
    } else {
        Err(Error::Invalid {
            message: format!("federated alias {alias:?} must match [A-Za-z_][A-Za-z0-9_]*"),
        })
    }
}
