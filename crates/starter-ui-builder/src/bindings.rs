//! Typed binding-expression helpers.
//!
//! SDUI bindings are template strings like `{{$page.severity}}` that
//! the resolver substitutes at render time. Hand-typed they're easy to
//! mis-spell; this module wraps the common families behind tiny
//! constructors that can't be misremembered.
//!
//! All helpers return `String` (not a newtype) — bindings flow into
//! arbitrary string fields (table queries, labels, etc.), and the
//! resolver doesn't care whether the source emitted them via this
//! helper or via raw `format!`. The point is authoring ergonomics, not
//! a wire-shape change.
//!
//! Child-walk (`/<name>`) compositions are not exposed as a separate
//! helper — append the `/segment` text to the returned binding (e.g.
//! `format!("{}/temp.value", target("path"))`). The resolver's
//! grammar is documented in `starter-ui-bindings`.
//!
//! # Example
//!
//! ```
//! use starter_ui_builder::bindings::{page_state, self_, user};
//!
//! assert_eq!(page_state("severity"), "{{$page.severity}}");
//! assert_eq!(user("email"), "{{$user.email}}");
//! assert_eq!(self_("layout"), "{{$self.layout}}");
//! ```

/// `{{$page.<key>}}` — read a key off the page-state envelope. Used
/// for select / date-range / search-input bindings.
pub fn page_state(key: impl AsRef<str>) -> String {
    format!("{{{{$page.{}}}}}", key.as_ref())
}

/// `{{$stack[<n>].<field>}}` — read a slot off the n-th frame of the
/// navigation stack (0 is the bottom-most frame).
pub fn stack(index: usize, field: impl AsRef<str>) -> String {
    format!("{{{{$stack[{}].{}}}}}", index, field.as_ref())
}

/// `{{$user.<field>}}` — read a JWT user-claim. Common claims:
/// `email`, `roles[]`, `subject`.
pub fn user(field: impl AsRef<str>) -> String {
    format!("{{{{$user.{}}}}}", field.as_ref())
}

/// `{{$self.<field>}}` — read a slot off the page node itself. Useful
/// when an authored page templatises its own configuration.
pub fn self_(field: impl AsRef<str>) -> String {
    format!("{{{{$self.{}}}}}", field.as_ref())
}

/// `{{$target.<field>}}` — read a slot off the bound target node.
/// Used in kind-default views (rendered via `/ui/render`) and in pages
/// that mount with a target frame on the stack.
pub fn target(field: impl AsRef<str>) -> String {
    format!("{{{{$target.{}}}}}", field.as_ref())
}

/// `{{$vars.<key>}}` — read a layout-scoped constant declared in the
/// `vars` block of the [`starter_ui_ir::ComponentTree`].
pub fn vars(key: impl AsRef<str>) -> String {
    format!("{{{{$vars.{}}}}}", key.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_state_wraps_key() {
        assert_eq!(page_state("severity"), "{{$page.severity}}");
    }

    #[test]
    fn stack_includes_index() {
        assert_eq!(stack(0, "id"), "{{$stack[0].id}}");
        assert_eq!(stack(2, "name"), "{{$stack[2].name}}");
    }

    #[test]
    fn user_and_self_and_target_and_vars() {
        assert_eq!(user("email"), "{{$user.email}}");
        assert_eq!(self_("layout"), "{{$self.layout}}");
        assert_eq!(target("status"), "{{$target.status}}");
        assert_eq!(vars("api_base"), "{{$vars.api_base}}");
    }
}
