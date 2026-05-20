//! Catalog format + loader. See SCOPE.md Phase 3.
//!
//! Stage 12 lands an empty module; the next stage adds the
//! `Catalog` struct, the `deny_unknown_fields` deserialiser, the
//! sha256-hex-prefix fingerprinter, and the platform seed loader.
