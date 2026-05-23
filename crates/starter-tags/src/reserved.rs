//! T6 — reserved keys, as code.

/// A reserved key plus its documented meaning.
#[derive(Clone, Copy, Debug)]
pub struct ReservedKey {
    pub key: &'static str,
    pub meaning: &'static str,
}

/// The full T6 table. Two sources of truth (this table + the SCOPE
/// markdown) is the point; the docs are normative for humans, this
/// constant is normative for code.
pub const RESERVED_KEYS: &[ReservedKey] = &[
    ReservedKey {
        key: "kind",
        meaning: "Entity kind (point, equip, site, flow, page, …)",
    },
    ReservedKey {
        key: "unit",
        meaning: "Canonical unit string (degC, kWh, m/s, …)",
    },
    ReservedKey {
        key: "source",
        meaning: "Where the row originated (mqtt, bacnet, flow:<id>, …)",
    },
    ReservedKey {
        key: "entityRef",
        meaning: "Generic ref. Use a specific name (equipRef, siteRef) when the relation is known.",
    },
];

/// True if `key` is one of the workspace-reserved keys.
pub fn is_reserved(key: &str) -> bool {
    RESERVED_KEYS.iter().any(|r| r.key == key)
}
