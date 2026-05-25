//! `Bindable` trait + `ResolveIssue` enum.
//!
//! The trait is the contract walkers depend on: every component in
//! the resolved tree exposes its id, its (optional) binding, and a
//! way to install a resolved slot value. The hand-written impl on
//! `Component` (in this crate, in `component.rs`) forwards the first
//! two methods to the `#[derive(Bindable)]` output and owns
//! `set_resolved_value` — the per-variant coerce policy.
//!
//! Why a single trait, three concerns:
//!   - `id()` — WritePlan and ResolveIssue both need a stable
//!     reference to the component that produced them. Returning
//!     `Option<&str>` accommodates both required (`id: String`,
//!     bound controls) and optional (`id: Option<String>`, layout
//!     containers) shapes without forcing layout authors to invent
//!     ids they will never use.
//!   - `binding()` — the structural test "does this variant carry a
//!     `BindingSpec`". Wrapping the answer in an `Option` lets
//!     walkers run uniformly over every variant; layout-only
//!     variants short-circuit on `None`.
//!   - `set_resolved_value()` — coerce a raw `serde_json::Value`
//!     (read from a slot) to the variant's expected shape and
//!     install it. Type mismatch pushes a `ResolveIssue` and leaves
//!     `value` unset — falling back to the variant's static default
//!     is preferable to painting `null`, per SDUI-VALUES.md §5.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::BindingSpec;

/// Read-back / resolution issues surfaced by `Bindable::
/// set_resolved_value` and the surrounding walker.
///
/// Three structurally distinct kinds, kept as separate variants so
/// Studio and the resolver can branch on them without parsing free
/// text. The `component_id` is the active `Component`'s `id()` value
/// at the time the issue was emitted, or `None` for variants whose
/// `id` is itself optional and unset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResolveIssue {
    /// The bind expression doesn't resolve to any concrete
    /// `(path, slot)` — target node missing, slot doesn't exist on
    /// the kind, expression malformed. The component was *wired*
    /// wrong; no value can be read back.
    DanglingBind {
        component_id: Option<String>,
        bind: String,
        reason: String,
    },
    /// The bind expression resolves cleanly but no value has ever
    /// been written to that slot. Distinguished from
    /// `DanglingBind` so Studio can offer a different remedy
    /// ("the binding is fine; the runtime hasn't ticked yet").
    UnsetSlot {
        component_id: Option<String>,
        path: String,
        slot: String,
    },
    /// The slot resolved and produced a value, but the value's
    /// JSON shape doesn't match what the bound variant expects
    /// (e.g. slider got a string). The component leaves `value`
    /// unset and falls back to its declared default.
    TypeMismatch {
        component_id: Option<String>,
        expected: &'static str,
        got: String,
    },
}

impl ResolveIssue {
    /// Convenience constructor — the dangling-bind variant is the
    /// most common emitter and ergonomic call sites are worth a
    /// helper.
    pub fn dangling_bind(
        component_id: Option<&str>,
        bind: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::DanglingBind {
            component_id: component_id.map(str::to_owned),
            bind: bind.into(),
            reason: reason.into(),
        }
    }

    /// Convenience constructor — the resolver knows the resolved
    /// `(path, slot)` pair when it discovers an unset slot.
    pub fn unset_slot(
        component_id: Option<&str>,
        path: impl Into<String>,
        slot: impl Into<String>,
    ) -> Self {
        Self::UnsetSlot {
            component_id: component_id.map(str::to_owned),
            path: path.into(),
            slot: slot.into(),
        }
    }

    /// Convenience constructor — `expected` is a `&'static str`
    /// so the variant impl can pass a literal like `"number"` or
    /// `"bool"`; `got` is rendered from the actual JSON shape.
    pub fn type_mismatch(
        component_id: Option<&str>,
        expected: &'static str,
        got: impl Into<String>,
    ) -> Self {
        Self::TypeMismatch {
            component_id: component_id.map(str::to_owned),
            expected,
            got: got.into(),
        }
    }
}

/// Trait implemented by `Component` (via a hand-written impl that
/// forwards to the `#[derive(Bindable)]` output). See module docs
/// for the rationale behind each method.
///
/// Read and write are split per SDUI-VALUES.md §3.1: writes are
/// inherently plural (a control may fan out user input to N slots),
/// reads are inherently singular (a widget displays one value at a
/// time). The trait does not implicitly default `read_binding` to
/// `write_bindings[0]` — variant impls write the convention by hand
/// so a future "input mirror" widget can deviate without fighting
/// hidden defaults.
pub trait Bindable {
    /// Stable component id. `None` for variants that declare
    /// `id: Option<String>` and were authored without one.
    fn id(&self) -> Option<&str>;

    /// What slot drives the displayed value. Singular by design —
    /// a widget renders one value at a time. The common
    /// implementation is `self.bind.0.first()` for variants that
    /// carry a `Bindings` field; layout-only variants return `None`.
    /// Independent of `write_bindings()`; see SDUI-VALUES.md §3.1.
    fn read_binding(&self) -> Option<&BindingSpec>;

    /// All slots this control writes to on user input. Empty slice
    /// for variants without a `Bindings` field. Each entry becomes
    /// one `WritePlan` entry, in declaration order. Per
    /// SDUI-VALUES.md §3.1, propagation is per-entry — there is no
    /// batching guarantee across the array, and partial failure
    /// (ACL, OCC, ensure_node) is handled per the §5 policy.
    fn write_bindings(&self) -> &[BindingSpec];

    /// Coerce a raw JSON value (read from the slot the binding
    /// resolved to) and install it as the variant's resolved
    /// `value`. On type mismatch, push a `ResolveIssue` and leave
    /// `value` unset — the renderer then falls back to the
    /// variant's static default rather than painting `null`.
    ///
    /// Implementations are expected to be no-ops on variants that
    /// do not declare a `value` field; the trait keeps the method
    /// total so walkers can call it uniformly without prior
    /// dispatch.
    fn set_resolved_value(&mut self, value: JsonValue, issues: &mut Vec<ResolveIssue>);

    /// Visit every `{{...}}`-bearing string field on **this node only**
    /// (does not recurse into children), allowing the visitor to
    /// rewrite the string in place. The substitute walker calls this
    /// once per node as it descends the tree, so child traversal stays
    /// explicit in the walker rather than implicit in every variant.
    ///
    /// Layout-only variants whose only mutable strings are CSS-token
    /// fields (gap, columns) must NOT pass those — the visitor is
    /// exclusively for fields that may carry a binding tag.
    fn visit_bindings<F>(&mut self, visit: &mut F)
    where
        F: FnMut(&mut String);
}

/// JSON shape names used by `ResolveIssue::TypeMismatch`. Centralised
/// so the strings stay consistent between the variant impls and the
/// resolver tests; if a renderer or Studio panel grows a switch on
/// `expected`, it has one source of truth.
pub mod expected_shape {
    pub const BOOL: &str = "bool";
    pub const NUMBER: &str = "number";
    pub const STRING: &str = "string";
}

/// Render the JSON value's shape as a stable lowercase string for
/// `ResolveIssue::TypeMismatch::got`. Mirrors what `serde_json`
/// already calls these internally; centralising the names keeps
/// the resolver's diagnostic strings predictable.
pub fn json_shape(v: &JsonValue) -> &'static str {
    match v {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "bool",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}
