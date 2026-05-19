//! # starter-service-slack
//!
//! Inbound Slack integration as a [`Service`](starter_spi::service::Service).
//! Opens a [Socket Mode](https://api.slack.com/apis/socket-mode) WebSocket
//! via `apps.connections.open`, deserializes each envelope, and emits the
//! Slack-side event into the [`EventSink`](starter_spi::service::EventSink)
//! the consumer wired into [`ServiceContext`](starter_spi::service::ServiceContext).
//!
//! ## SCOPE rules this crate honours
//!
//! - **R1** — one integration per crate. Slack outbound lives in
//!   `starter-tool-slack`; this crate is *only* the inbound side.
//! - **R2** — `Service`, not `Tool`. The trait shape is in `starter-spi`.
//!   The registry owns the `watch::Sender<bool>`; this crate observes
//!   `ctx.shutdown` and exits cooperatively.
//! - **R4** — the service does **not** pattern-match on payloads beyond
//!   what `serde_json` already does to recognise the envelope shape.
//!   Every events_api event becomes `kind = "slack.<event_type>"` plus
//!   the deserialized JSON body. Domain interpretation lives in the
//!   consumer.
//! - **R5** — `app_token` arrives as a [`SecretString`]. The crate does
//!   not read env vars or files.
//! - **R7** — observability is required. A restart counter, an
//!   event-emitted counter, and an is-running gauge are registered on
//!   `ctx.metrics`; every interesting transition emits a structured
//!   `tracing` event with a stable `service.name` field.
//! - **R8** — no transitive vendor SDKs in `starter-spi`. The only
//!   starter-side deps here are `starter-spi` and
//!   `starter-observability`.
//! - **R9** — restart policy is the service's, not the registry's. The
//!   inner connect loop is wrapped in an exponential-backoff retry layer
//!   with a max-attempts circuit so a permanently broken Slack app does
//!   not pin the service in a hot loop. After the circuit trips the
//!   service exits and the `JoinHandle` resolves with an `Internal`
//!   error; the registry observes that and stops there (R9 — no
//!   auto-restart).
//!
//! Implementation mined from `codeless/crates/codeless-slack`'s
//! `socket_mode.rs` and `envelope.rs`; the lifecycle framing (Service,
//! ServiceContext, EventSink) is starter's.
//!
//! ## Quick start
//!
//! ```no_run
//! use std::sync::Arc;
//! use prometheus::Registry;
//! use starter_spi::SecretString;
//! use starter_spi::service::ServiceRegistry;
//! use starter_service_slack::{SlackSocketModeConfig, SlackSocketModeService};
//!
//! # async fn ex() -> Result<(), Box<dyn std::error::Error>> {
//! # let sink: Arc<dyn starter_spi::service::EventSink> = unimplemented!();
//! let registry = Arc::new(Registry::new());
//! let cfg = SlackSocketModeConfig {
//!     app_token: SecretString::from("xapp-…".to_string()),
//!     base_url:  SlackSocketModeConfig::default_base_url(),
//! };
//! let svc = SlackSocketModeService::new(cfg);
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
mod metrics;
mod retry;
mod service;
mod socket_mode;

pub use config::SlackSocketModeConfig;
pub use error::SlackSocketModeError;
pub use retry::RetryPolicy;
pub use service::{SlackSocketModeService, SERVICE_NAME};
