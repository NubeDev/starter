//! sqlite-backed [`ThemeStore`].

use async_trait::async_trait;
use sqlx::Row;
use starter_spi::error::Error;
use starter_spi::ui::theme::{ShellConfig, ThemeDocument, ThemeSaveInput, ThemeStore, ThemeStyles};
use starter_store_sqlite::Pool;

use crate::asset_urls;

/// sqlite-backed [`ThemeStore`]. Single-row `starter_ui_theme` table
/// — run the `ui_theme_sqlite` migrations before constructing.
pub struct SqliteThemeStore {
    pool: Pool,
}

impl SqliteThemeStore {
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

fn parse_json<T: serde::de::DeserializeOwned + Default>(raw: Option<String>) -> Result<T, Error> {
    match raw {
        Some(s) => serde_json::from_str(&s).map_err(err),
        None => Ok(T::default()),
    }
}

async fn ensure_row(pool: &Pool) -> Result<(), Error> {
    sqlx::query("INSERT OR IGNORE INTO starter_ui_theme (id) VALUES (1)")
        .execute(pool.sqlx())
        .await
        .map_err(err)?;
    Ok(())
}

#[async_trait]
impl ThemeStore for SqliteThemeStore {
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

        let styles: String = row.get(0);
        let shell: String = row.get(1);
        let has_logo: i64 = row.get(2);
        let has_favicon: i64 = row.get(3);

        Ok(ThemeDocument {
            theme_styles: parse_json::<ThemeStyles>(Some(styles))?,
            shell: parse_json::<ShellConfig>(Some(shell))?,
            logo_url: (has_logo != 0).then(|| asset_urls::LOGO.to_string()),
            favicon_url: (has_favicon != 0).then(|| asset_urls::FAVICON.to_string()),
        })
    }

    async fn save(&self, input: ThemeSaveInput) -> Result<ThemeDocument, Error> {
        ensure_row(&self.pool).await?;
        let styles = serde_json::to_string(&input.theme_styles).map_err(err)?;
        let shell = serde_json::to_string(&input.shell).map_err(err)?;
        sqlx::query(
            "UPDATE starter_ui_theme \
             SET theme_styles = ?1, shell = ?2, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE id = 1",
        )
        .bind(&styles)
        .bind(&shell)
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        self.load().await
    }

    async fn put_logo(&self, bytes: Vec<u8>, content_type: &str) -> Result<String, Error> {
        ensure_row(&self.pool).await?;
        sqlx::query(
            "UPDATE starter_ui_theme \
             SET logo_bytes = ?1, logo_mime = ?2, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
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
             SET logo_bytes = NULL, logo_mime = NULL, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
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
             SET favicon_bytes = ?1, favicon_mime = ?2, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
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
             SET favicon_bytes = NULL, favicon_mime = NULL, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
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
