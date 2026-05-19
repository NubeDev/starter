//! `ThemeStore` implementations.
//!
//! Each backend is feature-gated so a consumer compiling against
//! sqlite never pulls Postgres deps (and vice versa). The
//! [`starter_spi::ui::theme::ThemeStore`] trait stays the public
//! seam — handlers hold `Arc<dyn ThemeStore>`.

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteThemeStore;

#[cfg(feature = "postgres")]
pub use postgres::PostgresThemeStore;
