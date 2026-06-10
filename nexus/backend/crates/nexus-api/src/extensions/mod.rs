//! WS-14 — Extensions runtime: mount the `starter-extensions` kernel into
//! nexus-api (host + lifecycle + cleanup + capability host-methods).
//!
//! The `starter-extensions` kernel is a near-complete extension subsystem (a
//! manifest registry, an OTP-style supervisor with a process-group reaper, an
//! admin HTTP surface with ETag-cached Module-Federation bundle serving, a
//! Postgres enablement store, a cleanup mechanism, and a Vite federation host
//! for the UI). This module is the **integration**: it
//!
//! - **boots** the kernel ([`boot`]) — reap orphans → scan/validate/seal the
//!   registry → materialise contributed query-kinds → spawn enabled supervisors
//!   → assemble the [`ExtensionAdmin`];
//! - **contributes** an extension's `warehouse_templates[]` as nexus
//!   query-kinds (the dispatcher's third source — [`contribute`],
//!   [`post_install`]) and reclaims them on uninstall ([`cleanup`]);
//! - **audits** every enable/disable/install/uninstall into `nexus_changes`
//!   with the acting principal ([`audit`], via the kernel's new audit sink);
//! - **mounts** the admin router under `/api/v1/extensions/*` ([`router`]); and
//! - **answers capability host-methods** so a process-flavour extension can call
//!   back into nexus (`warehouse.query` / `authz.check` / `dashboard.read`)
//!   under the caller's tenant, never broader than its grants ([`host_methods`]).
//!
//! ## Boot ordering (the chicken/egg)
//!
//! The host-method handler ([`host_methods::NexusHostMethods`]) closes over
//! [`AppState`], and `AppState` holds the `extension_kinds` registry. So `main`
//! must:
//! 1. [`boot::load_extension_kinds`] → persist + build the `extension_kinds`
//!    registry,
//! 2. build `AppState` with that registry,
//! 3. build the host-method handler over that `AppState`,
//! 4. [`boot::boot`] with the handler → the [`ExtensionAdmin`],
//! 5. [`router::router`] → mount, and shut supervisors down at exit.

pub mod audit;
pub mod boot;
pub mod cleanup;
pub mod config;
pub mod contribute;
pub mod host_methods;
pub mod post_install;
pub mod router;

pub use boot::{boot, load_extension_kinds, ExtensionRuntime};
pub use config::ExtensionsConfig;
pub use host_methods::NexusHostMethods;
pub use router::router;
