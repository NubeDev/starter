//! [`SqliteSetupRunStore`] — the thin run index over flow `RunId`s
//! (DOCS §4/§8).

use async_trait::async_trait;
use sqlx::Row;
use starter_flow_spi::flow::RunId;
use starter_setup_spi::error::{SetupError, SetupResult};
use starter_setup_spi::model::{
    Progress, SemVer, SetupRun, SetupRunStatus, TemplateId,
};
use starter_setup_spi::store::{SetupRunFilter, SetupRunStore};

use crate::pool::Pool;

/// SQLite-backed [`SetupRunStore`].
#[derive(Clone)]
pub struct SqliteSetupRunStore {
    pool: Pool,
}

impl SqliteSetupRunStore {
    /// Construct over an existing [`Pool`].
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn backend(e: sqlx::Error) -> SetupError {
    SetupError::Backend(e.to_string())
}

fn row_to_run(row: &sqlx::sqlite::SqliteRow) -> SetupResult<SetupRun> {
    let run_id_s: String = row.try_get("run_id").map_err(backend)?;
    let run_id = run_id_s
        .parse::<uuid::Uuid>()
        .map(RunId)
        .map_err(|e| SetupError::Backend(format!("run_id: {e}")))?;
    let template_id: String = row.try_get("template_id").map_err(backend)?;
    let template_ver: String = row.try_get("template_ver").map_err(backend)?;
    let owner: String = row.try_get("owner").map_err(backend)?;
    let tenant_id: Option<String> = row.try_get("tenant_id").map_err(backend)?;
    let team: Option<String> = row.try_get("team").map_err(backend)?;
    let status_s: String = row.try_get("status").map_err(backend)?;
    let progress_s: String = row.try_get("progress_json").map_err(backend)?;
    let failed_node: Option<String> = row.try_get("failed_node").map_err(backend)?;
    let resumable_i: i64 = row.try_get("resumable").map_err(backend)?;
    let created_at: String = row.try_get("created_at").map_err(backend)?;
    let finished_at: Option<String> = row.try_get("finished_at").map_err(backend)?;

    Ok(SetupRun {
        run_id,
        template_id: TemplateId(template_id),
        template_version: SemVer::parse(&template_ver)?,
        owner,
        tenant_id,
        team,
        status: SetupRunStatus::parse(&status_s)
            .ok_or_else(|| SetupError::Backend(format!("bad status: {status_s}")))?,
        progress: serde_json::from_str(&progress_s)
            .map_err(|e| SetupError::Backend(format!("progress: {e}")))?,
        failed_node,
        resumable: resumable_i != 0,
        created_at,
        finished_at,
    })
}

#[async_trait]
impl SetupRunStore for SqliteSetupRunStore {
    async fn record(&self, run: SetupRun) -> SetupResult<()> {
        let pool = self.pool.sqlx();
        let progress = serde_json::to_string(&run.progress)
            .map_err(|e| SetupError::Backend(format!("progress: {e}")))?;
        sqlx::query(
            "INSERT INTO setup_runs \
             (run_id, template_id, template_ver, owner, tenant_id, team, status, \
              progress_json, failed_node, resumable, created_at, finished_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )
        .bind(run.run_id.0.to_string())
        .bind(run.template_id.0)
        .bind(run.template_version.to_string())
        .bind(run.owner)
        .bind(run.tenant_id)
        .bind(run.team)
        .bind(run.status.as_str())
        .bind(progress)
        .bind(run.failed_node)
        .bind(run.resumable as i64)
        .bind(run.created_at)
        .bind(run.finished_at)
        .execute(pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn get(&self, run_id: RunId) -> SetupResult<Option<SetupRun>> {
        let pool = self.pool.sqlx();
        let row = sqlx::query("SELECT * FROM setup_runs WHERE run_id = ?1")
            .bind(run_id.0.to_string())
            .fetch_optional(pool)
            .await
            .map_err(backend)?;
        row.map(|r| row_to_run(&r)).transpose()
    }

    async fn list(&self, filter: SetupRunFilter) -> SetupResult<Vec<SetupRun>> {
        let pool = self.pool.sqlx();
        // Dynamic predicate; bind in a stable order.
        let mut sql = String::from("SELECT * FROM setup_runs WHERE 1=1");
        if filter.owner.is_some() {
            sql.push_str(" AND owner = ?");
        }
        if filter.tenant_id.is_some() {
            sql.push_str(" AND tenant_id = ?");
        }
        if filter.template_id.is_some() {
            sql.push_str(" AND template_id = ?");
        }
        if filter.status.is_some() {
            sql.push_str(" AND status = ?");
        }
        sql.push_str(" ORDER BY created_at DESC");
        let mut q = sqlx::query(&sql);
        if let Some(o) = &filter.owner {
            q = q.bind(o);
        }
        if let Some(t) = &filter.tenant_id {
            q = q.bind(t);
        }
        if let Some(t) = &filter.template_id {
            q = q.bind(&t.0);
        }
        if let Some(s) = &filter.status {
            q = q.bind(s.as_str());
        }
        let rows = q.fetch_all(pool).await.map_err(backend)?;
        rows.iter().map(row_to_run).collect()
    }

    async fn update_progress(
        &self,
        run_id: RunId,
        progress: Progress,
        status: SetupRunStatus,
    ) -> SetupResult<()> {
        let pool = self.pool.sqlx();
        let progress_s = serde_json::to_string(&progress)
            .map_err(|e| SetupError::Backend(format!("progress: {e}")))?;
        sqlx::query("UPDATE setup_runs SET progress_json = ?1, status = ?2 WHERE run_id = ?3")
            .bind(progress_s)
            .bind(status.as_str())
            .bind(run_id.0.to_string())
            .execute(pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn mark_failed(
        &self,
        run_id: RunId,
        failed_node: Option<String>,
        resumable: bool,
    ) -> SetupResult<()> {
        let pool = self.pool.sqlx();
        sqlx::query(
            "UPDATE setup_runs SET status = 'Failed', failed_node = ?1, resumable = ?2, \
             finished_at = COALESCE(finished_at, CURRENT_TIMESTAMP) WHERE run_id = ?3",
        )
        .bind(failed_node)
        .bind(resumable as i64)
        .bind(run_id.0.to_string())
        .execute(pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn mark_finished(
        &self,
        run_id: RunId,
        status: SetupRunStatus,
        finished_at: String,
    ) -> SetupResult<()> {
        let pool = self.pool.sqlx();
        sqlx::query("UPDATE setup_runs SET status = ?1, finished_at = ?2 WHERE run_id = ?3")
            .bind(status.as_str())
            .bind(finished_at)
            .bind(run_id.0.to_string())
            .execute(pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn list_open(&self) -> SetupResult<Vec<RunId>> {
        let pool = self.pool.sqlx();
        // Pending/Running are crash-recovery candidates (§8a); Failed rows
        // are resume candidates only when resumable (§8b auto-recovery).
        let rows = sqlx::query(
            "SELECT run_id FROM setup_runs \
             WHERE status IN ('Pending', 'Running') \
                OR (status = 'Failed' AND resumable = 1)",
        )
        .fetch_all(pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|r| {
                let s: String = r.try_get("run_id").map_err(backend)?;
                s.parse::<uuid::Uuid>()
                    .map(RunId)
                    .map_err(|e| SetupError::Backend(format!("run_id: {e}")))
            })
            .collect()
    }
}
