//! Postgres-backed [`ThemeStore`]. Same shape as the sqlite impl —
//! single-row `starter_ui_theme` table, assets stored as BYTEA.

use async_trait::async_trait;
use sqlx::Row;
use starter_spi::error::Error;
use starter_spi::ui::theme::{ShellConfig, ThemeDocument, ThemeSaveInput, ThemeStore, ThemeStyles};
use starter_store_postgres::Pool;

use crate::asset_urls;

/// Postgres-backed [`ThemeStore`]. Run the `ui_theme_postgres`
/// migrations before constructing.
pub struct PostgresThemeStore {
    pool: Pool,
}

impl PostgresThemeStore {
    /// Wrap the pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn err(e: impl std::error::Error + Send + Sync + 'static) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

async fn ensure_row(pool: &Pool) -> Result<(), Error> {
    sqlx::query("INSERT INTO starter_ui_theme (id) VALUES (1) ON CONFLICT (id) DO NOTHING")
        .execute(pool.sqlx())
        .await
        .map_err(err)?;
    Ok(())
}

#[async_trait]
impl ThemeStore for PostgresThemeStore {
    async fn load(&self) -> Result<ThemeDocument, Error> {
        let row = sqlx::query(
            "SELECT theme_styles, shell, \
                    (logo_bytes IS NOT NULL) AS has_logo, \
                    (favicon_bytes IS NOT NULL) AS has_favicon \
             FROM starter_ui_theme WHERE id = 1",
        )
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(err)?;

        let Some(row) = row else {
            return Ok(ThemeDocument::default());
        };

        let styles: sqlx::types::Json<ThemeStyles> = row.get(0);
        let shell: sqlx::types::Json<ShellConfig> = row.get(1);
        let has_logo: bool = row.get(2);
        let has_favicon: bool = row.get(3);

        Ok(ThemeDocument {
            theme_styles: styles.0,
            shell: shell.0,
            logo_url: has_logo.then(|| asset_urls::LOGO.to_string()),
            favicon_url: has_favicon.then(|| asset_urls::FAVICON.to_string()),
        })
    }

    async fn save(&self, input: ThemeSaveInput) -> Result<ThemeDocument, Error> {
        ensure_row(&self.pool).await?;
        sqlx::query(
            "UPDATE starter_ui_theme \
             SET theme_styles = $1::jsonb, shell = $2::jsonb, updated_at = NOW() \
             WHERE id = 1",
        )
        .bind(sqlx::types::Json(&input.theme_styles))
        .bind(sqlx::types::Json(&input.shell))
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        self.load().await
    }

    async fn put_logo(&self, bytes: Vec<u8>, content_type: &str) -> Result<String, Error> {
        ensure_row(&self.pool).await?;
        sqlx::query(
            "UPDATE starter_ui_theme \
             SET logo_bytes = $1, logo_mime = $2, updated_at = NOW() \
             WHERE id = 1",
        )
        .bind(bytes)
        .bind(content_type)
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(asset_urls::LOGO.to_string())
    }

    async fn delete_logo(&self) -> Result<(), Error> {
        sqlx::query(
            "UPDATE starter_ui_theme \
             SET logo_bytes = NULL, logo_mime = NULL, updated_at = NOW() \
             WHERE id = 1",
        )
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(())
    }

    async fn put_favicon(&self, bytes: Vec<u8>, content_type: &str) -> Result<String, Error> {
        ensure_row(&self.pool).await?;
        sqlx::query(
            "UPDATE starter_ui_theme \
             SET favicon_bytes = $1, favicon_mime = $2, updated_at = NOW() \
             WHERE id = 1",
        )
        .bind(bytes)
        .bind(content_type)
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(asset_urls::FAVICON.to_string())
    }

    async fn delete_favicon(&self) -> Result<(), Error> {
        sqlx::query(
            "UPDATE starter_ui_theme \
             SET favicon_bytes = NULL, favicon_mime = NULL, updated_at = NOW() \
             WHERE id = 1",
        )
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(())
    }

    async fn read_logo(&self) -> Result<Option<(Vec<u8>, String)>, Error> {
        let row = sqlx::query("SELECT logo_bytes, logo_mime FROM starter_ui_theme WHERE id = 1")
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(err)?;
        Ok(row.and_then(|r| {
            let bytes: Option<Vec<u8>> = r.get(0);
            let mime: Option<String> = r.get(1);
            match (bytes, mime) {
                (Some(b), Some(m)) => Some((b, m)),
                _ => None,
            }
        }))
    }

    async fn read_favicon(&self) -> Result<Option<(Vec<u8>, String)>, Error> {
        let row =
            sqlx::query("SELECT favicon_bytes, favicon_mime FROM starter_ui_theme WHERE id = 1")
                .fetch_optional(self.pool.sqlx())
                .await
                .map_err(err)?;
        Ok(row.and_then(|r| {
            let bytes: Option<Vec<u8>> = r.get(0);
            let mime: Option<String> = r.get(1);
            match (bytes, mime) {
                (Some(b), Some(m)) => Some((b, m)),
                _ => None,
            }
        }))
    }
}
