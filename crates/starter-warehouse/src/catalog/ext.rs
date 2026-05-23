//! W12 — extension trust seam.
//!
//! When an extension's manifest hash changes, every live
//! mart/cleaner the extension authored is re-quarantined in the
//! same transaction. The hash gate also decides whether a freshly
//! defined mart lands `pending` (auto-promotes to `live` on DDL
//! success) or `quarantined` (operator must `mart.promote`).

use sqlx::{PgConnection, Postgres, Transaction};

use super::CatalogError;

/// Author-type classification for the W12 lifecycle table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Author<'a> {
    User(&'a str),
    Agent(&'a str),
    Ext { id: &'a str, manifest_hash: &'a str },
}

impl<'a> Author<'a> {
    /// Parse `created_by` text + an optional manifest hash for
    /// `ext:` rows. Returns [`CatalogError::BadCreatedBy`] when the
    /// prefix is unrecognised.
    pub fn parse(
        created_by: &'a str,
        manifest_hash: Option<&'a str>,
    ) -> Result<Self, CatalogError> {
        if let Some(rest) = created_by.strip_prefix("user:") {
            Ok(Author::User(rest))
        } else if let Some(rest) = created_by.strip_prefix("agent:") {
            Ok(Author::Agent(rest))
        } else if let Some(rest) = created_by.strip_prefix("ext:") {
            let h = manifest_hash.unwrap_or("");
            Ok(Author::Ext {
                id: rest,
                manifest_hash: h,
            })
        } else {
            Err(CatalogError::BadCreatedBy(created_by.to_string()))
        }
    }

    pub fn created_by(&self) -> String {
        match self {
            Author::User(u) => format!("user:{u}"),
            Author::Agent(a) => format!("agent:{a}"),
            Author::Ext { id, .. } => format!("ext:{id}"),
        }
    }
}

/// Initial status decision per the W12 author-type table.
///
/// - `user:…` → `pending` (auto-promote on DDL success).
/// - `agent:…` → `quarantined` (explicit `mart.promote`).
/// - `ext:<id>` → `pending` iff the (ext_id, manifest_hash) pair is
///   already in `ext_manifest_approvals`; otherwise `quarantined`.
pub async fn initial_status<'c>(
    conn: &mut PgConnection,
    author: &Author<'c>,
) -> Result<&'static str, CatalogError> {
    match author {
        Author::User(_) => Ok("pending"),
        Author::Agent(_) => Ok("quarantined"),
        Author::Ext { id, manifest_hash } => {
            let approved: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ext_manifest_approvals \
                 WHERE ext_id = $1 AND manifest_hash = $2",
            )
            .bind(id)
            .bind(manifest_hash)
            .fetch_optional(&mut *conn)
            .await?;
            Ok(if approved.is_some() {
                "pending"
            } else {
                "quarantined"
            })
        }
    }
}

/// W12 manifest-hash re-quarantine. When an extension ships a new
/// manifest, every live mart and cleaner authored by it is moved
/// back to `quarantined` in the *same* transaction the new mart
/// definition runs in — there is no window in which a stale
/// manifest's marts remain `live` after the new manifest is seen.
pub async fn requarantine_for_ext(
    tx: &mut Transaction<'_, Postgres>,
    ext_id: &str,
) -> Result<RequarantineReport, CatalogError> {
    let prefix = format!("ext:{ext_id}");
    let m = sqlx::query(
        "UPDATE marts SET status = 'quarantined' \
         WHERE created_by = $1 AND status IN ('pending','live')",
    )
    .bind(&prefix)
    .execute(&mut **tx)
    .await?;
    let c = sqlx::query(
        "UPDATE cleaners SET status = 'quarantined' \
         WHERE created_by = $1 AND status IN ('pending','live')",
    )
    .bind(&prefix)
    .execute(&mut **tx)
    .await?;
    Ok(RequarantineReport {
        marts: m.rows_affected(),
        cleaners: c.rows_affected(),
    })
}

/// Counts moved by [`requarantine_for_ext`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RequarantineReport {
    pub marts: u64,
    pub cleaners: u64,
}

/// Insert an approval row. Idempotent on
/// `(ext_id, manifest_hash)`.
pub async fn record_approval(
    conn: &mut PgConnection,
    ext_id: &str,
    manifest_hash: &str,
    approved_by: &str,
) -> Result<(), CatalogError> {
    sqlx::query(
        "INSERT INTO ext_manifest_approvals (ext_id, manifest_hash, approved_by) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (ext_id, manifest_hash) DO NOTHING",
    )
    .bind(ext_id)
    .bind(manifest_hash)
    .bind(approved_by)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
