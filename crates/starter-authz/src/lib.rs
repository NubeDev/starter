//! # starter-authz
//!
//! Policy-based authorization layer over the
//! [`starter_spi::auth::Principal`] produced by whichever
//! `Authenticator` the binary wires in. Implements the
//! [`starter_spi::authz::PolicyEngine`] trait and ships the
//! default RBAC-with-ownership engine, a static (TOML-backed)
//! registry, and an axum [`require_permission`] middleware.
//!
//! This crate is **strictly optional** (workspace R5). A consumer
//! that does not depend on it pays nothing — no engine
//! constructed, no admin routes mounted, no migrations.
//!
//! See `DOCS/auth/authz/SCOPE.md` for the full scope.
//!
//! ## Quick start
//!
//! ```no_run
//! use std::sync::Arc;
//! use starter_authz::{StaticRbacEngine, StaticRegistry, AuthzConfig};
//! use starter_spi::authz::{Ownership, ResourceSpec};
//!
//! let registry = Arc::new(StaticRegistry::new());
//! registry.register_spec(ResourceSpec::from_static(
//!     "flows",
//!     &["read", "create", "update", "delete"],
//!     Ownership::Subject,
//!     "Flows",
//!     "User-authored automation flows.",
//! ));
//!
//! let cfg = AuthzConfig::default();
//! let engine: Arc<StaticRbacEngine> =
//!     Arc::new(StaticRbacEngine::from_config(cfg, registry.clone()).unwrap());
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod audit;
pub mod condition;
pub mod config;
pub mod defaults;
pub mod engine;
pub mod error;
pub mod instances;
pub mod middleware;
pub mod registry;
pub mod surface;
pub mod testing;

#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub mod acl;

#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub mod grants;

pub use surface::{current_surface, with_surface};

pub use audit::{DecisionEntry, DecisionSink, NoopDecisionSink};

#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub use audit::{DbDecisionSink, DecisionSinkConfig, RetentionConfig};

#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub mod db_engine;
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub mod routes;
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub mod store;

pub use condition::Expr;
pub use config::{Assignment, AuthzConfig, Effect, Rule};
pub use engine::StaticRbacEngine;
pub use error::Error;
pub use middleware::{require_permission, with_permission, with_permission_owned};
pub use registry::StaticRegistry;

#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub use db_engine::DbPolicyEngine;
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub use routes::authz_router;
