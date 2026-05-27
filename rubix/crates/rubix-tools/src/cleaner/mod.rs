//! L1→L2 cleaner: anomaly-rule trait, builtin rules, registry.
//!
//! The cleaner reads short windows of raw L1 samples per
//! `(tenant_id, entity_id)`, applies a sequence of [`AnomalyRule`]s
//! to each row, and emits a row into the L2 hypertable with the
//! winning quality tag.
//!
//! ## Why a trait instead of a function bag
//!
//! Each rule is a tiny stateless detector — but extensions need to
//! contribute their own rules from outside this crate. The trait is
//! the seam:
//!
//! - **Builtin rules** ([`builtin`]) impl `AnomalyRule` directly
//!   and run in-process — no JSON-RPC overhead per row.
//! - **Extension rules** (`#3b`) wrap a contributed tool dispatch
//!   inside an `AnomalyRule` adapter; the cleaner doesn't know the
//!   rule's origin.
//!
//! ## Shape: row + window + outcome
//!
//! [`AnomalyRule::apply`] sees one row plus a window of preceding
//! rows in chronological order. Window content is the **same
//! `(tenant_id, entity_id)`** the row carries — rules never cross
//! tenant or entity boundaries (`run_tick` keeps that invariant at
//! the level above).
//!
//! Outcomes ([`RuleOutcome`]) are: `Ok` (pass through), `Flag` (set
//! `quality` to the rule's verdict + optional note), `Drop` (skip
//! the row in L2). Rules run in registration order; the **first
//! non-Ok outcome wins** and short-circuits the remaining rules for
//! that row. Operators control the order via the registry's add
//! sequence.

pub mod builtin;
pub mod registry;
pub mod rule;

pub use builtin::{NanRule, SpikeRule, StuckRule};
pub use registry::RuleRegistry;
pub use rule::{AnomalyRule, QualityTag, Reading, RuleOutcome, WindowSlice};
