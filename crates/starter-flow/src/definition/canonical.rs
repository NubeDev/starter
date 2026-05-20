//! RFC 8785 (JCS) canonicalisation + `blake3` content hashing.
//!
//! `DOCS/flow/scope/hot-reload.md` HR1 step 2 mandates JCS:
//! *"two editors that emit semantically-equal JSON in different key
//! orders must produce the same hash, otherwise the idempotent
//! short-circuit silently breaks and `FlowStore` accretes duplicate
//! revisions."*
//!
//! JSON Canonicalisation Scheme rules implemented here:
//!
//! - UTF-8 output, no BOM.
//! - Object keys serialised in lexicographic order of their UTF-16
//!   code units (RFC 8785 §3.2.3).
//! - No insignificant whitespace.
//! - Numbers serialised per ECMA-262 number-to-string (RFC 8785
//!   §3.2.2.3) — for the JSON value space we accept
//!   (`serde_json::Value::Number`), this reduces to `ryu`-style
//!   shortest-round-trip output for floats and direct decimal for
//!   integers. We delegate to `serde_json`'s built-in number
//!   formatting which already meets this contract for the integer +
//!   `f64` subset `serde_json::Number` supports; arbitrary-precision
//!   numbers (`arbitrary_precision` feature) are *not* enabled in
//!   this workspace, so we don't have to worry about that case.
//! - Strings serialised with the minimal JSON string escapes
//!   (`\"`, `\\`, `\b`, `\f`, `\n`, `\r`, `\t`, `\uXXXX` for control
//!   chars; no unnecessary `\u` escapes for non-ASCII).
//!
//! The implementation walks the parsed `serde_json::Value` rather
//! than the input text — a body that round-trips through
//! `serde_json::from_str` already has its primitive representations
//! normalised by the parser, so the canonicaliser only has to
//! enforce key ordering, whitespace, and string-escape rules.
//!
//! HR1 step 2 uses `blake3` over the canonical bytes; the hash is
//! 32 bytes, surfaced as a lowercase hex string for log / audit /
//! comparison ergonomics.

use std::cmp::Ordering;

/// Hex-encoded 32-byte `blake3` hash of a flow-body's canonical
/// representation. Lowercase, no `0x` prefix, no separators —
/// suitable as a column value or a tracing field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BodyHash(String);

impl BodyHash {
    /// Borrow the hex string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Take the hex string out.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for BodyHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Canonicalise a `serde_json::Value` per RFC 8785.
///
/// Returns the canonical UTF-8 byte string. Pure; cheap; no I/O.
pub fn canonicalise(value: &serde_json::Value) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    write_value(value, &mut out);
    out
}

/// Convenience: canonicalise + `blake3` in one shot. The canonical
/// bytes are not retained.
pub fn body_hash(value: &serde_json::Value) -> BodyHash {
    let bytes = canonicalise(value);
    let hash = blake3::hash(&bytes);
    BodyHash(hash.to_hex().to_string())
}

fn write_value(value: &serde_json::Value, out: &mut Vec<u8>) {
    match value {
        serde_json::Value::Null => out.extend_from_slice(b"null"),
        serde_json::Value::Bool(true) => out.extend_from_slice(b"true"),
        serde_json::Value::Bool(false) => out.extend_from_slice(b"false"),
        serde_json::Value::Number(n) => {
            // `serde_json::Number`'s `Display` is ECMA-262-compatible
            // for the integer / f64 subset we accept; the workspace
            // does not enable the `arbitrary_precision` feature so
            // there are no other variants to worry about.
            out.extend_from_slice(n.to_string().as_bytes());
        }
        serde_json::Value::String(s) => write_string(s, out),
        serde_json::Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_value(item, out);
            }
            out.push(b']');
        }
        serde_json::Value::Object(map) => {
            // RFC 8785 §3.2.3: object members sorted by UTF-16 code
            // unit comparison of their keys. `serde_json::Map`
            // preserves insertion order by default, so we have to
            // re-sort here.
            let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
            keys.sort_unstable_by(utf16_cmp);
            out.push(b'{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_string(key, out);
                out.push(b':');
                let v = map.get(*key).expect("key came from this map");
                write_value(v, out);
            }
            out.push(b'}');
        }
    }
}

/// Lexicographic compare of two `&str` by their UTF-16 code units —
/// the comparison RFC 8785 §3.2.3 mandates for object key ordering.
///
/// Implemented by iterating the UTF-16 code units of each string in
/// lock-step; this is the minimum work to be spec-correct without
/// materialising an owned `Vec<u16>` per key per call.
fn utf16_cmp(a: &&str, b: &&str) -> Ordering {
    let mut ai = a.encode_utf16();
    let mut bi = b.encode_utf16();
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => match x.cmp(&y) {
                Ordering::Equal => continue,
                ne => return ne,
            },
        }
    }
}

fn write_string(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for ch in s.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{08}' => out.extend_from_slice(b"\\b"),
            '\u{0C}' => out.extend_from_slice(b"\\f"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            c if (c as u32) < 0x20 => {
                // Other control characters: \u00XX.
                let _ = std::io::Write::write_fmt(
                    &mut *out,
                    format_args!("\\u{:04x}", c as u32),
                );
            }
            c => {
                // RFC 8785 §3.2.2.2: non-ASCII characters are
                // emitted as raw UTF-8 (not escaped). `char::encode_utf8`
                // writes 1..4 bytes.
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                out.extend_from_slice(s.as_bytes());
            }
        }
    }
    out.push(b'"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_order_collapses_to_same_hash() {
        let a = serde_json::json!({"b": 1, "a": 2});
        let b = serde_json::json!({"a": 2, "b": 1});
        assert_eq!(canonicalise(&a), canonicalise(&b));
        assert_eq!(body_hash(&a), body_hash(&b));
    }

    #[test]
    fn nested_objects_sort_recursively() {
        let a = serde_json::json!({"outer": {"y": 2, "x": 1}});
        let b = serde_json::json!({"outer": {"x": 1, "y": 2}});
        assert_eq!(canonicalise(&a), canonicalise(&b));
    }

    #[test]
    fn arrays_preserve_order() {
        let a = serde_json::json!([3, 1, 2]);
        let b = serde_json::json!([1, 2, 3]);
        assert_ne!(canonicalise(&a), canonicalise(&b));
    }

    #[test]
    fn whitespace_is_stripped() {
        let from_text: serde_json::Value =
            serde_json::from_str("  {  \"a\" :  1 ,  \"b\" : 2 }  ").unwrap();
        let canon = canonicalise(&from_text);
        assert_eq!(std::str::from_utf8(&canon).unwrap(), r#"{"a":1,"b":2}"#);
    }

    #[test]
    fn strings_escape_control_chars_minimally() {
        let v = serde_json::json!("a\nb\tc\u{1}");
        let canon = canonicalise(&v);
        assert_eq!(std::str::from_utf8(&canon).unwrap(), r#""a\nb\tc\u0001""#);
    }

    #[test]
    fn non_ascii_strings_emit_raw_utf8() {
        let v = serde_json::json!("café");
        let canon = canonicalise(&v);
        assert_eq!(std::str::from_utf8(&canon).unwrap(), "\"café\"");
    }

    #[test]
    fn body_hash_is_lowercase_hex_64_chars() {
        let h = body_hash(&serde_json::json!({}));
        assert_eq!(h.as_str().len(), 64);
        assert!(h.as_str().chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn utf16_key_order_matches_rfc_example() {
        // RFC 8785 §3.2.3 example: "\u005C" (`\`) sorts before
        // "\u0061" (`a`) in UTF-16 code-unit order.
        let v = serde_json::json!({"a": 1, "\\": 2});
        let canon = std::str::from_utf8(&canonicalise(&v)).unwrap().to_owned();
        // `\` (0x5C) < `a` (0x61) → backslash key comes first.
        assert!(canon.starts_with(r#"{"\\":2,"a":1}"#));
    }
}
