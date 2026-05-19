//! Domain types + the `NoteStore` repository. Nothing here is
//! aware of axum, tonic, MCP, or clap — every surface re-uses these
//! same primitives. That's the point of the demo: one domain layer,
//! many surfaces.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Note {
    pub id: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateNote {
    pub body: String,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NoteError {
    #[error("note not found: {0}")]
    NotFound(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

#[derive(Clone)]
pub struct NoteStore {
    pool: SqlitePool,
}

impl NoteStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, body: &str, created_by: &str) -> Result<Note, NoteError> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = Utc::now();
        let created_at_s = created_at.to_rfc3339();
        sqlx::query("INSERT INTO notes (id, body, created_at, created_by) VALUES (?1, ?2, ?3, ?4)")
            .bind(&id)
            .bind(body)
            .bind(&created_at_s)
            .bind(created_by)
            .execute(&self.pool)
            .await?;
        Ok(Note {
            id,
            body: body.to_string(),
            created_at,
            created_by: created_by.to_string(),
        })
    }

    pub async fn get(&self, id: &str) -> Result<Note, NoteError> {
        let row: Option<(String, String, String, String)> =
            sqlx::query_as("SELECT id, body, created_at, created_by FROM notes WHERE id = ?1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        match row {
            Some((id, body, created_at, created_by)) => Ok(Note {
                id,
                body,
                created_at: parse_dt(&created_at),
                created_by,
            }),
            None => Err(NoteError::NotFound(id.to_string())),
        }
    }

    pub async fn list(&self) -> Result<Vec<Note>, NoteError> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT id, body, created_at, created_by FROM notes ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, body, created_at, created_by)| Note {
                id,
                body,
                created_at: parse_dt(&created_at),
                created_by,
            })
            .collect())
    }

    pub async fn delete(&self, id: &str) -> Result<(), NoteError> {
        let res = sqlx::query("DELETE FROM notes WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(NoteError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub async fn search(&self, needle: &str) -> Result<Vec<Note>, NoteError> {
        let pattern = format!("%{needle}%");
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT id, body, created_at, created_by FROM notes WHERE body LIKE ?1 ORDER BY created_at DESC",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, body, created_at, created_by)| Note {
                id,
                body,
                created_at: parse_dt(&created_at),
                created_by,
            })
            .collect())
    }
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
