//! In-process matcher (T8c). The semantic oracle for T8a/T8b.

use crate::query::TagQuery;
use crate::set::{TagSet, TagValue};

/// Run `q` against `set`. Truth semantics:
///
/// * `Has(k)` matches iff the set has a `Bool(true)` at `k` (T3 sugar).
/// * `Eq(k, Bool(b))` matches iff the set has exactly `Bool(b)` at `k`;
///   `Str("true")` is **not** an implicit boolean (D6).
/// * `Eq(k, Str(s))` matches iff the set has exactly `Str(s)` at `k`,
///   byte-for-byte. No whitespace folding, no numeric normalisation.
pub fn matches(q: &TagQuery, set: &TagSet) -> bool {
    match q {
        TagQuery::Has(k) => matches!(set.get(k), Some(TagValue::Bool(true))),
        TagQuery::Eq(k, v) => match (set.get(k), v) {
            (Some(TagValue::Bool(a)), TagValue::Bool(b)) => a == b,
            (Some(TagValue::Str(a)), TagValue::Str(b)) => a == b,
            _ => false,
        },
        TagQuery::And(xs) => xs.iter().all(|x| matches(x, set)),
        TagQuery::Or(xs) => xs.iter().any(|x| matches(x, set)),
        TagQuery::Not(x) => !matches(x, set),
    }
}

/// Convenience: produce a boxed `Fn` for callers that want to stash a
/// compiled matcher (flow nodes, in-memory filters).
pub fn compile_to_match(q: &TagQuery) -> Box<dyn Fn(&TagSet) -> bool + Send + Sync> {
    let owned = q.clone();
    Box::new(move |s: &TagSet| matches(&owned, s))
}
