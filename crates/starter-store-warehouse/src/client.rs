//! Connection wrapper. The store crate owns the connection so the
//! W8 `async_insert=1 / wait_for_async_insert=1` discipline is
//! applied centrally — every write goes through a client built by
//! this module.

use clickhouse::Client;

/// Connection config. `url` is the HTTP endpoint
/// (`http://host:8123`); `database`, `user`, `password` follow the
/// ClickHouse Rust client conventions. `async_insert` defaults to
/// `true` per W8; disable only for `bulk.import` (W8a).
#[derive(Clone, Debug)]
pub struct ChConfig {
    pub url: String,
    pub database: String,
    pub user: String,
    pub password: String,
    /// `true` ⇒ set `async_insert=1, wait_for_async_insert=1` on
    /// every write. The W8 default. Set `false` ONLY for
    /// `bulk.import` paths that batch in-engine (W8a).
    pub async_insert: bool,
}

impl ChConfig {
    /// Convenience: anonymous default-database connection to a
    /// local HTTP endpoint. Mostly used by the testcontainer
    /// helper; production callers build a [`ChConfig`] directly.
    pub fn local(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            database: "default".into(),
            user: "default".into(),
            password: String::new(),
            async_insert: true,
        }
    }
}

/// Thin wrapper around `clickhouse::Client`. Holds the inner
/// client by value; `Clone` is cheap (the inner is `Arc`-shared
/// internally).
#[derive(Clone)]
pub struct ChClient {
    inner: Client,
    cfg: ChConfig,
}

impl ChClient {
    /// Build a client and bake the W8 settings in. Subsequent
    /// per-query overrides via `.with_option(...)` are the only
    /// sanctioned escape hatch — and the only sanctioned use is
    /// `bulk.import` flipping to `async_insert=0`.
    pub fn connect(cfg: ChConfig) -> Self {
        let mut client = Client::default()
            .with_url(&cfg.url)
            .with_user(&cfg.user)
            .with_password(&cfg.password)
            .with_database(&cfg.database);
        if cfg.async_insert {
            client = client
                .with_option("async_insert", "1")
                .with_option("wait_for_async_insert", "1");
        }
        Self { inner: client, cfg }
    }

    /// Borrow the underlying client. Public so DDL-only consumers
    /// (migration runner, dashboards that need bespoke SELECTs,
    /// integration tests) can drive raw `query()` without a wrapper
    /// for each call shape. The W8 invariant ("no raw `INSERT INTO
    /// (raw_events|samples|events|documents)` outside `src/store/`")
    /// is enforced at CI lint level — a grep, not a visibility
    /// modifier — because gating at the type system level would
    /// force us to wrap every read query too. The trade-off is
    /// documented in W8 and the store/mod.rs prelude.
    pub fn inner(&self) -> &Client {
        &self.inner
    }

    /// The config the client was built with, for diagnostics.
    pub fn config(&self) -> &ChConfig {
        &self.cfg
    }
}

/// Errors surfaced by client / store calls.
#[derive(Debug, thiserror::Error)]
pub enum ChClientError {
    #[error("clickhouse: {0}")]
    Clickhouse(#[from] clickhouse::error::Error),
    #[error("serde_json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("other: {0}")]
    Other(String),
}
