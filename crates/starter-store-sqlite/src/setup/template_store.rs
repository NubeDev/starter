//! [`SqliteTemplateStore`] — the template catalog (DOCS §5).
//!
//! PK is `(tenant_id, id, version)` with the `__global__` sentinel. The
//! read path prefers a tenant row over a `__global__` row of the same
//! `(id, version)`.

use async_trait::async_trait;
use sqlx::Row;
use starter_setup_spi::error::{SetupError, SetupResult};
use starter_setup_spi::model::{
    SemVer, Template, TemplateAccess, TemplateId, TemplateSource, TemplateSummary,
};
use starter_setup_spi::store::{TemplateFilter, TemplateStore, GLOBAL_TENANT_SENTINEL};

use super::StoredBindings;
use crate::pool::Pool;

/// SQLite-backed [`TemplateStore`].
#[derive(Clone)]
pub struct SqliteTemplateStore {
    pool: Pool,
}

impl SqliteTemplateStore {
    /// Construct over an existing [`Pool`]. Pair with
    /// [`super::SETUP_MIGRATION_SOURCE`] on the migrate chain.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn tenant_key(t: Option<&str>) -> &str {
    t.unwrap_or(GLOBAL_TENANT_SENTINEL)
}

fn ser<T: serde::Serialize>(v: &T) -> SetupResult<String> {
    serde_json::to_string(v).map_err(|e| SetupError::Backend(format!("serialize: {e}")))
}

fn de<T: serde::de::DeserializeOwned>(col: &str, raw: &str) -> SetupResult<T> {
    serde_json::from_str(raw).map_err(|e| SetupError::Backend(format!("deserialize {col}: {e}")))
}

fn backend(e: sqlx::Error) -> SetupError {
    SetupError::Backend(e.to_string())
}

fn row_to_template(row: &sqlx::sqlite::SqliteRow) -> SetupResult<Template> {
    let id: String = row.try_get("id").map_err(backend)?;
    let version: String = row.try_get("version").map_err(backend)?;
    let display_name: String = row.try_get("display_name").map_err(backend)?;
    let description: String = row.try_get("description").map_err(backend)?;
    let icon: Option<String> = row.try_get("icon").map_err(backend)?;
    let category: String = row.try_get("category").map_err(backend)?;
    let input_schema_s: String = row.try_get("input_schema").map_err(backend)?;
    let flow_body_s: String = row.try_get("flow_body").map_err(backend)?;
    let bindings_s: String = row.try_get("bindings").map_err(backend)?;
    let access_s: String = row.try_get("access").map_err(backend)?;
    let source_s: String = row.try_get("source").map_err(backend)?;

    let bindings: StoredBindings = de("bindings", &bindings_s)?;
    let access: TemplateAccess = de("access", &access_s)?;
    let source: TemplateSource = de("source", &source_s)?;

    Ok(Template {
        id: TemplateId(id),
        version: SemVer::parse(&version)?,
        display_name,
        description,
        icon,
        category,
        input_schema: de("input_schema", &input_schema_s)?,
        flow_body: de("flow_body", &flow_body_s)?,
        input_bindings: bindings.input,
        output_bindings: bindings.output,
        access,
        source,
    })
}

#[async_trait]
impl TemplateStore for SqliteTemplateStore {
    async fn put(&self, template: Template) -> SetupResult<TemplateId> {
        let pool = self.pool.sqlx();
        let tenant = tenant_key(template.access.tenant_id.as_deref()).to_string();
        let bindings = StoredBindings {
            input: template.input_bindings.clone(),
            output: template.output_bindings.clone(),
        };
        sqlx::query(
            "INSERT INTO setup_templates \
             (tenant_id, id, version, display_name, description, icon, category, \
              input_schema, flow_body, bindings, access, source) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) \
             ON CONFLICT (tenant_id, id, version) DO UPDATE SET \
              display_name = excluded.display_name, \
              description  = excluded.description, \
              icon         = excluded.icon, \
              category     = excluded.category, \
              input_schema = excluded.input_schema, \
              flow_body    = excluded.flow_body, \
              bindings     = excluded.bindings, \
              access       = excluded.access, \
              source       = excluded.source",
        )
        .bind(&tenant)
        .bind(template.id.0.clone())
        .bind(template.version.to_string())
        .bind(&template.display_name)
        .bind(&template.description)
        .bind(&template.icon)
        .bind(&template.category)
        .bind(ser(&template.input_schema)?)
        .bind(ser(&template.flow_body)?)
        .bind(ser(&bindings)?)
        .bind(ser(&template.access)?)
        .bind(ser(&template.source)?)
        .execute(pool)
        .await
        .map_err(backend)?;
        Ok(template.id)
    }

    async fn get(
        &self,
        tenant_id: Option<&str>,
        id: &TemplateId,
        version: Option<SemVer>,
    ) -> SetupResult<Option<Template>> {
        let pool = self.pool.sqlx();
        // Candidate tenant keys, in preference order: the caller's tenant
        // first, then the global sentinel (DOCS §5 overlay model).
        let mut keys: Vec<String> = Vec::new();
        if let Some(t) = tenant_id {
            keys.push(t.to_string());
        }
        if keys.iter().all(|k| k != GLOBAL_TENANT_SENTINEL) {
            keys.push(GLOBAL_TENANT_SENTINEL.to_string());
        }

        for key in keys {
            let row = match &version {
                Some(v) => sqlx::query(
                    "SELECT * FROM setup_templates \
                     WHERE tenant_id = ?1 AND id = ?2 AND version = ?3",
                )
                .bind(&key)
                .bind(&id.0)
                .bind(v.to_string())
                .fetch_optional(pool)
                .await
                .map_err(backend)?,
                None => {
                    // "latest": order by parsed version. SQLite can't sort
                    // semver text correctly, so pull candidates and pick in
                    // Rust.
                    let rows = sqlx::query(
                        "SELECT * FROM setup_templates WHERE tenant_id = ?1 AND id = ?2",
                    )
                    .bind(&key)
                    .bind(&id.0)
                    .fetch_all(pool)
                    .await
                    .map_err(backend)?;
                    let mut best: Option<(SemVer, sqlx::sqlite::SqliteRow)> = None;
                    for r in rows {
                        let vs: String = r.try_get("version").map_err(backend)?;
                        let v = SemVer::parse(&vs)?;
                        if best.as_ref().map(|(bv, _)| v > *bv).unwrap_or(true) {
                            best = Some((v, r));
                        }
                    }
                    best.map(|(_, r)| r)
                }
            };
            if let Some(row) = row {
                return Ok(Some(row_to_template(&row)?));
            }
        }
        Ok(None)
    }

    async fn list(&self, filter: TemplateFilter) -> SetupResult<Vec<TemplateSummary>> {
        let pool = self.pool.sqlx();
        // Tenant rows plus inherited global rows. A tenant row hides the
        // global row of the same (id, version).
        let mut keys: Vec<String> = vec![GLOBAL_TENANT_SENTINEL.to_string()];
        if let Some(t) = &filter.tenant_id {
            if t != GLOBAL_TENANT_SENTINEL {
                keys.push(t.clone());
            }
        }
        let placeholders = (1..=keys.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let mut sql = format!(
            "SELECT tenant_id, id, version, display_name, category \
             FROM setup_templates WHERE tenant_id IN ({placeholders})"
        );
        if filter.category.is_some() {
            sql.push_str(&format!(" AND category = ?{}", keys.len() + 1));
        }
        let mut q = sqlx::query(&sql);
        for k in &keys {
            q = q.bind(k);
        }
        if let Some(c) = &filter.category {
            q = q.bind(c);
        }
        let rows = q.fetch_all(pool).await.map_err(backend)?;

        // Overlay: prefer tenant rows over global for the same (id,version).
        use std::collections::HashMap;
        let mut by_key: HashMap<(String, String), TemplateSummary> = HashMap::new();
        for r in rows {
            let tenant: String = r.try_get("tenant_id").map_err(backend)?;
            let id: String = r.try_get("id").map_err(backend)?;
            let version: String = r.try_get("version").map_err(backend)?;
            let display_name: String = r.try_get("display_name").map_err(backend)?;
            let category: String = r.try_get("category").map_err(backend)?;
            let is_global = tenant == GLOBAL_TENANT_SENTINEL;
            let summary = TemplateSummary {
                id: TemplateId(id.clone()),
                version: SemVer::parse(&version)?,
                display_name,
                category,
                tenant_id: if is_global { None } else { Some(tenant) },
            };
            let k = (id, version);
            match by_key.get(&k) {
                Some(existing) if existing.tenant_id.is_some() => {} // tenant wins
                _ => {
                    by_key.insert(k, summary);
                }
            }
        }
        Ok(by_key.into_values().collect())
    }

    async fn delete(
        &self,
        tenant_id: Option<&str>,
        id: &TemplateId,
        version: SemVer,
    ) -> SetupResult<()> {
        let pool = self.pool.sqlx();
        sqlx::query(
            "DELETE FROM setup_templates WHERE tenant_id = ?1 AND id = ?2 AND version = ?3",
        )
        .bind(tenant_key(tenant_id))
        .bind(&id.0)
        .bind(version.to_string())
        .execute(pool)
        .await
        .map_err(backend)?;
        Ok(())
    }
}
