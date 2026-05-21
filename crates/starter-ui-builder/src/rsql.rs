//! Typed RSQL composer.
//!
//! Authors compose RSQL filter strings via a fluent builder rather
//! than hand-concatenating `kind==X;path=p=*Y*` strings — the builder
//! eliminates the most common mistakes (forgetting `kind==`, mixing
//! `;` and `,`, mis-quoting). The output is the same RSQL the
//! resolver and `/ui/table` already accept; no new wire shape is
//! added.
//!
//! Per SCOPE.md R6 (and § Decisions S-D2), the RSQL grammar starter
//! emits is inherited from Rubix; a divergence row in DIVERGENCE.md
//! is reserved for the first operator that drifts.
//!
//! # Example
//!
//! ```
//! use starter_ui_builder::rsql::rsql;
//!
//! let q = rsql()
//!     .kind("com.acme.task")
//!     .eq("settings.gate", "open")
//!     .build();
//! assert_eq!(q, "kind==com.acme.task;settings.gate==open");
//! ```

/// Entry point for the typed RSQL composer.
///
/// All builder methods consume `self` and return `Self`, so the chain
/// reads top-to-bottom. [`RsqlBuilder::build`] yields an RSQL string;
/// [`RsqlBuilder::into_inner`] is identical and exists for callers
/// that prefer a verb that doesn't shadow common builder vocabulary.
pub fn rsql() -> RsqlBuilder {
    RsqlBuilder::default()
}

/// Builder for RSQL filter strings. See module-level docs.
///
/// Conjunctions are joined with `;` (logical AND), the form the
/// resolver's RSQL parser expects. Disjunctions and parenthesised
/// groups are not yet exposed — the typed surface intentionally
/// tracks what the resolver currently consumes; once the resolver
/// gains `,` support the builder will gain a matching `or` combinator.
#[derive(Debug, Default, Clone)]
pub struct RsqlBuilder {
    parts: Vec<String>,
}

impl RsqlBuilder {
    /// Add `kind==<id>` — the most common first clause.
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.parts.push(format!("kind=={}", kind.into()));
        self
    }

    /// Add `<field>==<value>`. Value is rendered without quoting; the
    /// resolver's RSQL parser accepts bare identifiers and quoted
    /// strings interchangeably for the common case of slot equality.
    pub fn eq(mut self, field: impl Into<String>, value: impl Into<String>) -> Self {
        self.parts
            .push(format!("{}=={}", field.into(), value.into()));
        self
    }

    /// Add `<field>!=<value>`.
    pub fn ne(mut self, field: impl Into<String>, value: impl Into<String>) -> Self {
        self.parts
            .push(format!("{}!={}", field.into(), value.into()));
        self
    }

    /// Add `<field>=p=*<pattern>*` — case-insensitive contains. Maps
    /// to the resolver's pattern operator without forcing the caller
    /// to remember the `=p=` glob spelling.
    pub fn contains(mut self, field: impl Into<String>, pattern: impl Into<String>) -> Self {
        self.parts
            .push(format!("{}=p=*{}*", field.into(), pattern.into()));
        self
    }

    /// Add `<field>=in=(a,b,c)`. Values are rendered comma-separated
    /// without quoting; pass already-escaped strings if a value can
    /// contain commas.
    pub fn in_set<I, S>(mut self, field: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let joined = values
            .into_iter()
            .map(|v| v.into())
            .collect::<Vec<_>>()
            .join(",");
        self.parts.push(format!("{}=in=({})", field.into(), joined));
        self
    }

    /// Restrict to descendants of a parent path. Maps to the
    /// resolver's `parent_path` predicate; useful when the page binds
    /// a target that owns a sub-tree (e.g. alarms hanging off a
    /// building).
    pub fn parent_path_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.parts
            .push(format!("parent_path=={}", prefix.into()));
        self
    }

    /// Append a raw RSQL fragment verbatim. Escape hatch — prefer the
    /// typed methods above. Useful when the resolver gains an
    /// operator the builder doesn't model yet.
    pub fn raw(mut self, fragment: impl Into<String>) -> Self {
        self.parts.push(fragment.into());
        self
    }

    /// True iff no clauses have been added.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Render to RSQL. Empty builders return an empty string —
    /// downstream consumers (`Rows::rsql`) treat that as "match all".
    pub fn build(self) -> String {
        self.parts.join(";")
    }

    /// Alias of [`Self::build`] for chains that already use `build`
    /// vocabulary on a parent builder.
    pub fn into_inner(self) -> String {
        self.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_builder_renders_empty_string() {
        assert_eq!(rsql().build(), "");
        assert!(rsql().is_empty());
    }

    #[test]
    fn kind_then_eq() {
        let q = rsql()
            .kind("com.acme.task")
            .eq("settings.gate", "open")
            .build();
        assert_eq!(q, "kind==com.acme.task;settings.gate==open");
    }

    #[test]
    fn contains_renders_pattern_operator() {
        let q = rsql().contains("path", "/proj/").build();
        assert_eq!(q, "path=p=*/proj/*");
    }

    #[test]
    fn in_set_renders_paren_list() {
        let q = rsql().in_set("kind", ["a", "b", "c"]).build();
        assert_eq!(q, "kind=in=(a,b,c)");
    }

    #[test]
    fn raw_appends_verbatim() {
        let q = rsql().kind("x").raw("custom=op=42").build();
        assert_eq!(q, "kind==x;custom=op=42");
    }
}
