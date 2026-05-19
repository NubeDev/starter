//! # starter-service-telegram
//!
//! Inbound Telegram integration as a
//! [`Service`](starter_spi::service::Service). Drives the Bot API's
//! [`getUpdates`](https://core.telegram.org/bots/api#getupdates)
//! long-poll loop, deserializes each update, and emits the
//! Telegram-side update into the
//! [`EventSink`](starter_spi::service::EventSink) the consumer wired
//! into [`ServiceContext`](starter_spi::service::ServiceContext).
//!
//! ## SCOPE rules this crate honours
//!
//! - **R1** — one integration per crate. Telegram outbound lives in
//!   `starter-tool-telegram`; this crate is *only* the inbound side.
//! - **R2** — `Service`, not `Tool`. The registry owns the
//!   `watch::Sender<bool>`; this crate observes `ctx.shutdown` and
//!   exits cooperatively at every await point.
//! - **R4** — the service does **not** pattern-match on payloads
//!   beyond what `serde_json` already does to recognise the update
//!   shape. Every `getUpdates` element becomes
//!   `kind = "telegram.<update_type>"` plus the deserialized JSON
//!   body verbatim. Domain interpretation lives in the consumer.
//! - **R5** — `bot_token` arrives as a
//!   [`SecretString`](starter_spi::SecretString). The crate does not
//!   read env vars or files.
//! - **R7** — observability is required. A restart counter, an
//!   event-emitted counter, and an is-running gauge are registered on
//!   `ctx.metrics`; every interesting transition emits a structured
//!   `tracing` event with a stable `service.name` field.
//! - **R8** — no transitive vendor SDKs in `starter-spi`. The only
//!   starter-side deps here are `starter-spi` and
//!   `starter-observability`.
//! - **R9** — restart policy is the service's, not the registry's.
//!   The inner long-poll loop is wrapped in an exponential-backoff
//!   retry layer with a max-attempts circuit so a permanently broken
//!   bot token (401) or wrong base URL (404) do not pin the service
//!   in a hot loop. After the circuit trips the service exits and the
//!   `JoinHandle` resolves with an `Internal` error; the registry
//!   observes that and stops there (R9 — no auto-restart).
//!
//! ## Update offset persistence (v0.1)
//!
//! Telegram acks every update at `getUpdates` time by sending
//! `offset = max(update_id) + 1` on the next call. This crate
//! persists that offset **in-memory** for v0.1 — restarting the
//! consumer binary re-delivers any update Telegram still has in its
//! retention window (24h). The [`OffsetStore`] trait is the seam an
//! at-rest backend (sqlite, redis, …) will slot into later without a
//! breaking change to the public surface.
//!
//! Implementation mined from
//! `codeless/crates/codeless-telegram::{web_api, long_poll}`; the
//! lifecycle framing (Service, ServiceContext, EventSink) is
//! starter's.
//!
//! ## Quick start
//!
//! ```no_run
//! use std::sync::Arc;
//! use prometheus::Registry;
//! use starter_spi::SecretString;
//! use starter_spi::service::ServiceRegistry;
//! use starter_service_telegram::{TelegramBotConfig, TelegramBotService};
//!
//! # async fn ex() -> Result<(), Box<dyn std::error::Error>> {
//! # let sink: Arc<dyn starter_spi::service::EventSink> = unimplemented!();
//! let registry = Arc::new(Registry::new());
//! let cfg = TelegramBotConfig {
//!     bot_token: SecretString::from("12345:abc…".to_string()),
//!     base_url:  TelegramBotConfig::default_base_url(),
//! };
//! let svc = TelegramBotService::new(cfg);
//!
//! let mut services = ServiceRegistry::new().register(svc);
//! services.start_all(registry, sink).await?;
//! // … run …
//! let _report = services.shutdown().await;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod config;
mod error;
mod long_poll;
mod metrics;
mod offset;
mod retry;
mod service;

pub use config::TelegramBotConfig;
pub use error::TelegramBotError;
pub use offset::{InMemoryOffsetStore, OffsetStore};
pub use retry::RetryPolicy;
pub use service::{TelegramBotService, SERVICE_NAME};
