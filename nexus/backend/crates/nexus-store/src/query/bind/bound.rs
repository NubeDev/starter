//! The output of the binder: rewritten SQL plus the bound argument vector.
//!
//! This is the project's injection + tenant-isolation boundary expressed as a
//! type. The runner executes [`BoundQuery`] as a prepared statement — every
//! [`SqlValue`] becomes a driver-bound `$N` parameter, never inlined text. The
//! only text the binder ever inserts into [`BoundQuery::sql`] is a vetted
//! identifier recorded in [`BoundQuery::validated_identifiers`]. See
//! docs/design/query/ for why values are bound, not quoted.

use chrono::{DateTime, Utc};

/// A single value bound into the query as a `$N` placeholder. The closed set
/// mirrors the column types the result side already speaks ([`super::super`]'s
/// `ResultColumnType`): time bounds, variable values, kind params, and host
/// tokens all land here as arguments the driver binds — they are never
/// concatenated into the SQL string.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    /// A text value (a `$var` value, a host token id, a `$__sqlIn` element).
    Text(String),
    /// A text array — the `$caller_team_ids` host token, bound from
    /// `Principal.teams` and consumed as `= ANY($N)`. Postgres binds a
    /// `Vec<String>` directly to a `text[]` parameter, so this never
    /// touches the SQL string (the same un-spoofable, fully-bound path the
    /// scalar tokens use).
    TextArray(Vec<String>),
    /// An integer value (a numeric variable/param).
    Int(i64),
    /// A floating-point value.
    Float(f64),
    /// A boolean value.
    Bool(bool),
    /// A UTC timestamp (a `$__timeFrom`/`$__timeTo` bound or a time variable).
    Timestamp(DateTime<Utc>),
    /// A SQL `NULL` (an explicitly null variable/param).
    Null,
}

/// A query rewritten for prepared execution.
///
/// `sql` carries `$1,$2,…` placeholders in order; `args[i]` is the value for
/// `$(i+1)`. `validated_identifiers` records every bare identifier or fragment
/// the binder inserted as text (each already vetted against the allowlist) so a
/// caller — or a test — can assert the text-insertion surface stayed tiny.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundQuery {
    /// Rewritten SQL with `$N` placeholders. Contains no interpolated values.
    pub sql: String,
    /// The bound arguments, positional: `args[i]` fills `$(i+1)`.
    pub args: Vec<SqlValue>,
    /// Identifiers/fragments inserted as text, each vetted. The audit trail of
    /// the one unavoidable non-bound path.
    pub validated_identifiers: Vec<String>,
}

impl BoundQuery {
    /// A binder accumulator: start empty and grow as the scan emits SQL text,
    /// placeholders, and identifiers.
    pub(crate) fn builder() -> BoundQueryBuilder {
        BoundQueryBuilder {
            sql: String::new(),
            args: Vec::new(),
            validated_identifiers: Vec::new(),
        }
    }
}

/// Incrementally assembles a [`BoundQuery`] as the scanner walks the input.
pub(crate) struct BoundQueryBuilder {
    sql: String,
    args: Vec<SqlValue>,
    validated_identifiers: Vec<String>,
}

impl BoundQueryBuilder {
    /// Append literal SQL text copied verbatim from the input (the spans between
    /// macros/variables). This text is author-controlled and is the existing
    /// guarded path — it is not a value-injection surface.
    pub(crate) fn push_sql(&mut self, text: &str) {
        self.sql.push_str(text);
    }

    /// Bind `value` as the next `$N` placeholder, writing `$N` into the SQL and
    /// appending the value to `args`. This is how every value reaches the query.
    pub(crate) fn push_arg(&mut self, value: SqlValue) {
        self.args.push(value);
        self.sql.push('$');
        self.sql.push_str(&self.args.len().to_string());
    }

    /// Insert an already-validated identifier/fragment as text and record it.
    /// Callers must have run it through the allowlist first — this method does
    /// not re-validate; it only tracks the insertion.
    pub(crate) fn push_identifier(&mut self, ident: &str) {
        self.sql.push_str(ident);
        self.validated_identifiers.push(ident.to_string());
    }

    /// Finish, yielding the immutable [`BoundQuery`].
    pub(crate) fn finish(self) -> BoundQuery {
        BoundQuery {
            sql: self.sql,
            args: self.args,
            validated_identifiers: self.validated_identifiers,
        }
    }
}
