//! Server-Driven UI — binding grammar, evaluator, and
//! subscription-plan derivation.
//!
//! Phase 2 of the SDUI port (DOCS/frontend/sdui/SCOPE.md). Ported
//! verbatim — in semantics, not in API surface — from
//! `rubix-agent/crates/dashboard-runtime` / `rubix-contracts/
//! dashboard-runtime`. Starter's grammar is the subset documented in
//! `SCOPE.md § Data bindings`:
//!
//! ```text
//! binding := source ( "." ident       # slot-read on cursor
//!                   | "/" ident       # child-walk (cursor move)
//!                   )*
//! source  := "$target" | "$stack" "." alias | "$self"
//!          | "$user"   | "$page"
//! ```
//!
//! `.` is data access — read a slot on the cursor's current node.
//! `/` is graph traversal — move the cursor to a named child.
//!
//! Evaluation is **length-prefixed**: the parser emits a flat list of
//! `(Op, ident)` steps in declaration order, the evaluator advances
//! one step at a time and the cursor's state at step *N* is fully
//! determined by steps *0..N*. This is what makes `{{$target/temp.value}}`
//! resolve identically against three different `target` entities —
//! the parsed expression has no entity-specific state.
//!
//! **D5** (DIVERGENCE.md): Rubix's binding engine resolves against
//! Rubix's specific node graph (nodeRef ref-walks, typed kinds, an
//! `InMemoryReader`); starter abstracts the host graph behind an
//! [`EntityGraph`] trait that consumers implement against whatever
//! they have. The grammar is unchanged; only the source of children
//! and slots is abstracted. Per **S-D1** (SCOPE.md § Decisions) the
//! trait lives in this crate until a second consumer wants it
//! promoted to `starter-spi`.

mod catalogue;
mod eval;
mod expand;
mod graph;
mod parse;
mod subscription;
mod substitute;

pub use catalogue::{MessageBag, NullBag};
pub use eval::{evaluate, BindingError, EvalContext};
pub use expand::{expand_repeats, ExpandError};
pub use graph::{ChildLink, EntityGraph, EntityId, NullGraph};
pub use parse::{Binding, ParseError, Qualifier, Source, Step};

pub use subscription::{SlotAccess, Subject, SubscriptionPlan};
pub use substitute::{substitute_text, substitute_tree};
