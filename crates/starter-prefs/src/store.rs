//! `PrefsStore` trait + sqlite implementation.
//!
//! Owns: SCOPE.md "Preferences model" + the storage entries in
//! "Crate layout". The storage layer is a **faithful mirror** of the
//! two tables — NULL on disk comes back as `None` in Rust, and no
//! implicit defaulting happens here. The R3 three-layer resolution
//! (and `"auto"` derivation) is the [`crate::resolver`]'s job; this
//! module only round-trips rows.
//!
//! The trait is always compiled; the [`SqlitePrefsStore`]
//! implementation is feature-gated behind the `sqlite` cargo
//! feature per workspace policy R5 and the Phase 1 decision lock
//! (sqlite-only for this job; Postgres is deferred to a follow-up).

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as JsonValue;
use starter_spi::error::Error;
use starter_spi::units::Unit;
use std::str::FromStr;

use crate::resolver::{OrgPrefsRow, StringPref, UnitPref, UserPrefsRow};

/// Embedded migrations for the sqlite backend. Apply via
/// [`SqlitePrefsStore::migrate`] before any other call.
///
/// Composing with `starter-store-sqlite`'s namespaced
/// migration runner is one struct literal away:
///
/// ```ignore
/// use starter_store_sqlite::MigrationSource;
///
/// const PREFS_SOURCE: MigrationSource = MigrationSource {
///     name: "starter_prefs",
///     migrator: &starter_prefs::store::MIGRATIONS,
/// };
/// ```
///
/// We intentionally do NOT export the `MigrationSource` constant
/// from this crate — that would invert the dependency direction
/// (starter-prefs would have to know about starter-store-sqlite,
/// which already depends only on `starter-spi`).
#[cfg(feature = "sqlite")]
pub static MIGRATIONS: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Persistence trait for starter-prefs rows.
///
/// All operations are keyed on `(user_id, workspace_id)` for the user
/// layer and `workspace_id` for the org layer per the SCOPE.md
/// "Preferences model" schema. `upsert_*` writes every column on the
/// row: `Option::None` becomes SQL `NULL` ("inherit"). Storage does
/// not coerce `None` into a default — that is the resolver's job
/// per R3.
#[async_trait]
pub trait PrefsStore: Send + Sync {
    /// Fetch the user-layer row for `(user_id, workspace_id)`. Returns
    /// `None` when no row exists (the user has never written prefs
    /// for that workspace).
    async fn get_user_prefs(
        &self,
        user_id: &str,
        workspace_id: &str,
    ) -> Result<Option<UserPrefsRow>, Error>;

    /// Fetch the org-layer row for `workspace_id`. Returns `None`
    /// when no row exists.
    async fn get_org_prefs(&self, workspace_id: &str) -> Result<Option<OrgPrefsRow>, Error>;

    /// Insert or update the user-layer row for
    /// `(user_id, workspace_id)`. Every column in `patch` is written:
    /// `None` becomes SQL `NULL`. `updated_at` is stamped server-side
    /// to the current UTC epoch in milliseconds.
    async fn upsert_user_prefs(
        &self,
        user_id: &str,
        workspace_id: &str,
        patch: UserPrefsRow,
    ) -> Result<(), Error>;

    /// Insert or update the org-layer row for `workspace_id`. Same
    /// semantics as [`Self::upsert_user_prefs`].
    async fn upsert_org_prefs(&self, workspace_id: &str, patch: OrgPrefsRow) -> Result<(), Error>;
}

// ---------------------------------------------------------------------
// DB <-> Row column codecs.
//
// Per-column helpers convert between the TEXT shape SCOPE.md
// "Preferences model" specifies and the typed enum / `UnitPref` /
// `StringPref` Rust shapes. Each pair preserves the round-trip
// requirement the Phase 1 integration tests check: NULL stays NULL,
// "auto" stays "auto", and an explicit value stays the same string.
// ---------------------------------------------------------------------

fn err(e: impl std::error::Error + Send + Sync + 'static) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

#[cfg_attr(not(feature = "sqlite"), allow(dead_code))]
fn enum_to_db<T: Serialize>(v: T) -> String {
    match serde_json::to_value(v).expect("enum serializes to JSON") {
        JsonValue::String(s) => s,
        other => panic!("expected JSON string for enum, got {other:?}"),
    }
}

#[cfg_attr(not(feature = "sqlite"), allow(dead_code))]
fn enum_from_db<T: DeserializeOwned>(s: &str) -> Result<T, Error> {
    serde_json::from_value::<T>(JsonValue::String(s.to_owned())).map_err(err)
}

#[cfg_attr(not(feature = "sqlite"), allow(dead_code))]
fn unit_pref_to_db(v: &UnitPref) -> String {
    match v {
        UnitPref::Auto => "auto".to_owned(),
        UnitPref::Explicit(u) => u.as_str().to_owned(),
    }
}

#[cfg_attr(not(feature = "sqlite"), allow(dead_code))]
fn unit_pref_from_db(s: &str) -> Result<UnitPref, Error> {
    if s == "auto" {
        Ok(UnitPref::Auto)
    } else {
        Unit::from_str(s).map(UnitPref::Explicit).map_err(|e| {
            err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid unit code {s:?}: {e}"),
            ))
        })
    }
}

#[cfg_attr(not(feature = "sqlite"), allow(dead_code))]
fn string_pref_to_db(v: &StringPref) -> String {
    match v {
        StringPref::Auto => "auto".to_owned(),
        StringPref::Explicit(s) => s.clone(),
    }
}

#[cfg_attr(not(feature = "sqlite"), allow(dead_code))]
fn string_pref_from_db(s: &str) -> StringPref {
    StringPref::parse(s)
}

// ---------------------------------------------------------------------
// Sqlite implementation (feature-gated).
// ---------------------------------------------------------------------

#[cfg(feature = "sqlite")]
mod sqlite_impl {
    use super::*;
    use sqlx::sqlite::SqliteRow;
    use sqlx::Row;
    use sqlx::SqlitePool;
    use starter_spi::preferences::{
        DateFormat, NumberFormat, Theme, TimeFormat, UnitSystem, WeekStart,
    };

    /// `sqlx::SqlitePool`-backed [`PrefsStore`].
    ///
    /// Construct from an existing pool, then call
    /// [`SqlitePrefsStore::migrate`] (or apply [`super::MIGRATIONS`]
    /// directly against the pool) before any other call.
    pub struct SqlitePrefsStore {
        pool: SqlitePool,
    }

    impl SqlitePrefsStore {
        /// Wrap a pool. Caller is responsible for migration; use
        /// [`Self::migrate`] for the convenience path.
        pub fn new(pool: SqlitePool) -> Self {
            Self { pool }
        }

        /// Apply the bundled migrations to the wrapped pool.
        pub async fn migrate(&self) -> Result<(), Error> {
            super::MIGRATIONS.run(&self.pool).await.map_err(err)
        }

        /// Borrow the underlying pool — handy for tests that want to
        /// poke the schema directly.
        pub fn pool(&self) -> &SqlitePool {
            &self.pool
        }
    }

    fn now_epoch_ms() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    fn opt_text(row: &SqliteRow, col: &str) -> Option<String> {
        row.try_get::<Option<String>, _>(col).ok().flatten()
    }

    fn opt_unit(row: &SqliteRow, col: &str) -> Result<Option<UnitPref>, Error> {
        opt_text(row, col)
            .map(|s| unit_pref_from_db(&s))
            .transpose()
    }

    fn opt_enum<T: DeserializeOwned>(row: &SqliteRow, col: &str) -> Result<Option<T>, Error> {
        opt_text(row, col)
            .map(|s| enum_from_db::<T>(&s))
            .transpose()
    }

    fn opt_string_pref(row: &SqliteRow, col: &str) -> Option<StringPref> {
        opt_text(row, col).map(|s| string_pref_from_db(&s))
    }

    fn decode_user_row(row: &SqliteRow) -> Result<UserPrefsRow, Error> {
        Ok(UserPrefsRow {
            timezone: opt_string_pref(row, "timezone"),
            locale: opt_text(row, "locale"),
            language: opt_text(row, "language"),
            unit_system: opt_enum::<UnitSystem>(row, "unit_system")?,
            temperature_unit: opt_unit(row, "temperature_unit")?,
            pressure_unit: opt_unit(row, "pressure_unit")?,
            speed_unit: opt_unit(row, "speed_unit")?,
            length_unit: opt_unit(row, "length_unit")?,
            mass_unit: opt_unit(row, "mass_unit")?,
            date_format: opt_enum::<DateFormat>(row, "date_format")?,
            time_format: opt_enum::<TimeFormat>(row, "time_format")?,
            week_start: opt_enum::<WeekStart>(row, "week_start")?,
            number_format: opt_enum::<NumberFormat>(row, "number_format")?,
            currency: opt_string_pref(row, "currency"),
            theme: opt_enum::<Theme>(row, "theme")?,
        })
    }

    fn decode_org_row(row: &SqliteRow) -> Result<OrgPrefsRow, Error> {
        Ok(OrgPrefsRow {
            timezone: opt_string_pref(row, "timezone"),
            locale: opt_text(row, "locale"),
            language: opt_text(row, "language"),
            unit_system: opt_enum::<UnitSystem>(row, "unit_system")?,
            temperature_unit: opt_unit(row, "temperature_unit")?,
            pressure_unit: opt_unit(row, "pressure_unit")?,
            speed_unit: opt_unit(row, "speed_unit")?,
            length_unit: opt_unit(row, "length_unit")?,
            mass_unit: opt_unit(row, "mass_unit")?,
            date_format: opt_enum::<DateFormat>(row, "date_format")?,
            time_format: opt_enum::<TimeFormat>(row, "time_format")?,
            week_start: opt_enum::<WeekStart>(row, "week_start")?,
            number_format: opt_enum::<NumberFormat>(row, "number_format")?,
            currency: opt_string_pref(row, "currency"),
        })
    }

    #[async_trait]
    impl PrefsStore for SqlitePrefsStore {
        async fn get_user_prefs(
            &self,
            user_id: &str,
            workspace_id: &str,
        ) -> Result<Option<UserPrefsRow>, Error> {
            let row = sqlx::query(
                "SELECT timezone, locale, language, unit_system, \
                        temperature_unit, pressure_unit, speed_unit, \
                        length_unit, mass_unit, date_format, \
                        time_format, week_start, number_format, \
                        currency, theme \
                 FROM starter_prefs_user \
                 WHERE user_id = ?1 AND workspace_id = ?2",
            )
            .bind(user_id)
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(err)?;
            row.as_ref().map(decode_user_row).transpose()
        }

        async fn get_org_prefs(&self, workspace_id: &str) -> Result<Option<OrgPrefsRow>, Error> {
            let row = sqlx::query(
                "SELECT timezone, locale, language, unit_system, \
                        temperature_unit, pressure_unit, speed_unit, \
                        length_unit, mass_unit, date_format, \
                        time_format, week_start, number_format, \
                        currency \
                 FROM starter_prefs_org \
                 WHERE workspace_id = ?1",
            )
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(err)?;
            row.as_ref().map(decode_org_row).transpose()
        }

        async fn upsert_user_prefs(
            &self,
            user_id: &str,
            workspace_id: &str,
            patch: UserPrefsRow,
        ) -> Result<(), Error> {
            sqlx::query(
                "INSERT INTO starter_prefs_user ( \
                    user_id, workspace_id, timezone, locale, language, \
                    unit_system, temperature_unit, pressure_unit, \
                    speed_unit, length_unit, mass_unit, date_format, \
                    time_format, week_start, number_format, currency, \
                    theme, updated_at \
                 ) VALUES ( \
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, \
                    ?13, ?14, ?15, ?16, ?17, ?18 \
                 ) \
                 ON CONFLICT(user_id, workspace_id) DO UPDATE SET \
                    timezone         = excluded.timezone, \
                    locale           = excluded.locale, \
                    language         = excluded.language, \
                    unit_system      = excluded.unit_system, \
                    temperature_unit = excluded.temperature_unit, \
                    pressure_unit    = excluded.pressure_unit, \
                    speed_unit       = excluded.speed_unit, \
                    length_unit      = excluded.length_unit, \
                    mass_unit        = excluded.mass_unit, \
                    date_format      = excluded.date_format, \
                    time_format      = excluded.time_format, \
                    week_start       = excluded.week_start, \
                    number_format    = excluded.number_format, \
                    currency         = excluded.currency, \
                    theme            = excluded.theme, \
                    updated_at       = excluded.updated_at",
            )
            .bind(user_id)
            .bind(workspace_id)
            .bind(patch.timezone.as_ref().map(string_pref_to_db))
            .bind(patch.locale.clone())
            .bind(patch.language.clone())
            .bind(patch.unit_system.map(enum_to_db))
            .bind(patch.temperature_unit.as_ref().map(unit_pref_to_db))
            .bind(patch.pressure_unit.as_ref().map(unit_pref_to_db))
            .bind(patch.speed_unit.as_ref().map(unit_pref_to_db))
            .bind(patch.length_unit.as_ref().map(unit_pref_to_db))
            .bind(patch.mass_unit.as_ref().map(unit_pref_to_db))
            .bind(patch.date_format.map(enum_to_db))
            .bind(patch.time_format.map(enum_to_db))
            .bind(patch.week_start.map(enum_to_db))
            .bind(patch.number_format.map(enum_to_db))
            .bind(patch.currency.as_ref().map(string_pref_to_db))
            .bind(patch.theme.map(enum_to_db))
            .bind(now_epoch_ms())
            .execute(&self.pool)
            .await
            .map_err(err)?;
            Ok(())
        }

        async fn upsert_org_prefs(
            &self,
            workspace_id: &str,
            patch: OrgPrefsRow,
        ) -> Result<(), Error> {
            sqlx::query(
                "INSERT INTO starter_prefs_org ( \
                    workspace_id, timezone, locale, language, \
                    unit_system, temperature_unit, pressure_unit, \
                    speed_unit, length_unit, mass_unit, date_format, \
                    time_format, week_start, number_format, currency, \
                    updated_at \
                 ) VALUES ( \
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, \
                    ?13, ?14, ?15, ?16 \
                 ) \
                 ON CONFLICT(workspace_id) DO UPDATE SET \
                    timezone         = excluded.timezone, \
                    locale           = excluded.locale, \
                    language         = excluded.language, \
                    unit_system      = excluded.unit_system, \
                    temperature_unit = excluded.temperature_unit, \
                    pressure_unit    = excluded.pressure_unit, \
                    speed_unit       = excluded.speed_unit, \
                    length_unit      = excluded.length_unit, \
                    mass_unit        = excluded.mass_unit, \
                    date_format      = excluded.date_format, \
                    time_format      = excluded.time_format, \
                    week_start       = excluded.week_start, \
                    number_format    = excluded.number_format, \
                    currency         = excluded.currency, \
                    updated_at       = excluded.updated_at",
            )
            .bind(workspace_id)
            .bind(patch.timezone.as_ref().map(string_pref_to_db))
            .bind(patch.locale.clone())
            .bind(patch.language.clone())
            .bind(patch.unit_system.map(enum_to_db))
            .bind(patch.temperature_unit.as_ref().map(unit_pref_to_db))
            .bind(patch.pressure_unit.as_ref().map(unit_pref_to_db))
            .bind(patch.speed_unit.as_ref().map(unit_pref_to_db))
            .bind(patch.length_unit.as_ref().map(unit_pref_to_db))
            .bind(patch.mass_unit.as_ref().map(unit_pref_to_db))
            .bind(patch.date_format.map(enum_to_db))
            .bind(patch.time_format.map(enum_to_db))
            .bind(patch.week_start.map(enum_to_db))
            .bind(patch.number_format.map(enum_to_db))
            .bind(patch.currency.as_ref().map(string_pref_to_db))
            .bind(now_epoch_ms())
            .execute(&self.pool)
            .await
            .map_err(err)?;
            Ok(())
        }
    }
}

#[cfg(feature = "sqlite")]
pub use sqlite_impl::SqlitePrefsStore;
