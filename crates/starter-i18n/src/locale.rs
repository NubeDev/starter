//! BCP-47-aware locale negotiation.
//!
//! Two free functions:
//!
//! - [`parse_accept_language`] — parses an `Accept-Language` header
//!   into a quality-ranked list of `(LanguageTag, q)` pairs. Garbage
//!   tags are silently dropped (the header is a hint, not a contract);
//!   the function returns an empty `Vec` if every entry is invalid.
//!   The wildcard `*` is RFC-permitted but is not itself a BCP-47
//!   language tag, so it does NOT appear in the parsed list;
//!   [`pick_language`] detects it directly from the raw header.
//! - [`pick_language`] — given the available catalog tags and the
//!   raw `Accept-Language` header, walks the SCOPE R5 fallback chain
//!   (requested exact → language family → wildcard → `fallback`).

use icu_locale_core::LanguageIdentifier;
use starter_spi::i18n::LanguageTag;

/// Parse an `Accept-Language` header into a quality-sorted list of
/// `(LanguageTag, q)` pairs.
///
/// Behaviour:
///
/// - Whitespace around commas and semicolons is tolerated.
/// - Missing `q=` defaults to `1.0` per RFC 7231.
/// - Out-of-range or unparseable `q` values fall back to `1.0`.
/// - Tags that fail BCP-47 validation are dropped silently — an
///   `Accept-Language` header is a hint, not a contract. This
///   includes the wildcard `*`; see [`pick_language`] for how the
///   wildcard is honoured.
/// - Entries are returned sorted by descending quality; the original
///   order is preserved within a quality band (stable sort).
pub fn parse_accept_language(header: &str) -> Vec<(LanguageTag, f32)> {
    let mut out: Vec<(LanguageTag, f32)> = Vec::new();

    for raw in header.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }

        let mut parts = raw.split(';');
        let Some(tag_str) = parts.next().map(str::trim) else {
            continue;
        };
        if tag_str.is_empty() {
            continue;
        }

        let mut q: f32 = 1.0;
        for param in parts {
            let param = param.trim();
            if let Some(rest) = param.strip_prefix("q=") {
                if let Ok(parsed) = rest.trim().parse::<f32>() {
                    if (0.0..=1.0).contains(&parsed) {
                        q = parsed;
                    }
                }
            }
        }

        // `*` is RFC-permitted but is not a BCP-47 tag — drop it
        // here and let `pick_language` honour the wildcard from the
        // raw header.
        let Ok(tag) = LanguageTag::parse(tag_str) else {
            continue;
        };
        out.push((tag, q));
    }

    // Stable sort by descending quality.
    out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
    out
}

/// Returns `true` if any comma-separated entry in `header` has `*`
/// as its tag part (ignoring `;q=…` parameters and whitespace).
fn header_has_wildcard(header: &str) -> bool {
    header
        .split(',')
        .map(|entry| entry.split(';').next().unwrap_or("").trim())
        .any(|tag| tag == "*")
}

/// Pick the best available `LanguageTag` for a request.
///
/// Implements the SCOPE R5 fallback chain:
///
/// 1. Walk the parsed `Accept-Language` list in quality order. For
///    each entry:
///    a. exact match against `available`, then
///    b. language-family match (`"en-US"` requested → any `"en-*"`
///       available; `"en"` requested → any `"en-*"` available).
/// 2. If the header contains `*`, return the first entry of
///    `available`.
/// 3. Otherwise return `fallback` (typically `"en"`).
///
/// `available` is taken in caller-supplied order; ties within an
/// `Accept-Language` quality band are broken by the order in
/// `available`.
pub fn pick_language(
    available: &[LanguageTag],
    accept: &str,
    fallback: LanguageTag,
) -> LanguageTag {
    if available.is_empty() {
        return fallback;
    }

    let parsed = parse_accept_language(accept);

    for (requested, _q) in &parsed {
        let req_str = requested.as_str();

        // 1a. Exact match.
        if let Some(found) = available.iter().find(|t| t.as_str() == req_str) {
            return found.clone();
        }

        // 1b. Language-family match. Compare the BCP-47 `language`
        // subtag — `"en-US"` and `"en-GB"` share family `"en"`. We
        // re-parse via icu_locale_core (cheap; the tag has already
        // been validated) to extract the family.
        let Ok(req_id) = LanguageIdentifier::try_from_str(req_str) else {
            continue;
        };
        let req_family = req_id.language;

        let family_match = available.iter().find(|t| {
            let Ok(avail_id) = LanguageIdentifier::try_from_str(t.as_str()) else {
                return false;
            };
            avail_id.language == req_family
        });
        if let Some(found) = family_match {
            return found.clone();
        }
    }

    // 2. Wildcard — RFC 7231 §5.3.5 lets the server pick anything.
    if header_has_wildcard(accept) {
        return available[0].clone();
    }

    // 3. Static fallback.
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(s: &str) -> LanguageTag {
        LanguageTag::parse(s).expect("test tag must parse")
    }

    // -------- parse_accept_language --------

    #[test]
    fn parses_single_tag() {
        let out = parse_accept_language("en-US");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0.as_str(), "en-US");
        assert_eq!(out[0].1, 1.0);
    }

    #[test]
    fn parses_quality_weight() {
        let out = parse_accept_language("en;q=0.5");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0.as_str(), "en");
        assert_eq!(out[0].1, 0.5);
    }

    #[test]
    fn sorts_by_descending_quality() {
        let out = parse_accept_language("en;q=0.5, fr;q=0.9, de");
        assert_eq!(out.len(), 3);
        // de has implicit q=1.0, then fr=0.9, then en=0.5.
        assert_eq!(out[0].0.as_str(), "de");
        assert_eq!(out[1].0.as_str(), "fr");
        assert_eq!(out[2].0.as_str(), "en");
    }

    #[test]
    fn parses_country_subtag() {
        let out = parse_accept_language("zh-TW");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0.as_str(), "zh-TW");
    }

    #[test]
    fn wildcard_is_dropped_from_parsed_list() {
        // `*` is honoured by pick_language directly; it doesn't
        // appear in the parsed (LanguageTag, q) list because `*` is
        // not itself a BCP-47 tag.
        let out = parse_accept_language("*");
        assert!(out.is_empty());
    }

    #[test]
    fn empty_header_yields_empty_vec() {
        assert!(parse_accept_language("").is_empty());
    }

    #[test]
    fn whitespace_only_yields_empty_vec() {
        assert!(parse_accept_language("   ").is_empty());
        assert!(parse_accept_language(" , , ").is_empty());
    }

    #[test]
    fn garbage_tags_are_dropped_silently() {
        // "en_US" uses underscore — invalid BCP-47. "!!!!" is garbage.
        // The valid "fr" survives.
        let out = parse_accept_language("en_US, !!!!, fr");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0.as_str(), "fr");
    }

    #[test]
    fn invalid_quality_value_falls_back_to_one() {
        let out = parse_accept_language("en;q=not-a-number");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].1, 1.0);
    }

    #[test]
    fn out_of_range_quality_falls_back_to_one() {
        let out = parse_accept_language("en;q=2.0");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].1, 1.0);
    }

    #[test]
    fn tolerates_extra_whitespace() {
        let out = parse_accept_language("  en-US ; q=0.8 , fr ");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0.as_str(), "fr"); // q=1.0
        assert_eq!(out[1].0.as_str(), "en-US");
        assert_eq!(out[1].1, 0.8);
    }

    // -------- pick_language --------

    #[test]
    fn pick_exact_match_wins() {
        let avail = vec![tag("en"), tag("es"), tag("fr")];
        let chosen = pick_language(&avail, "fr;q=0.9", tag("en"));
        assert_eq!(chosen.as_str(), "fr");
    }

    #[test]
    fn pick_family_match_when_no_exact() {
        // Request en-US; only en is available — family fallback.
        let avail = vec![tag("en"), tag("es")];
        let chosen = pick_language(&avail, "en-US", tag("en"));
        assert_eq!(chosen.as_str(), "en");
    }

    #[test]
    fn pick_family_match_picks_first_en_variant() {
        // Request "en"; "en-GB" is available — family fallback the
        // other direction.
        let avail = vec![tag("es"), tag("en-GB")];
        let chosen = pick_language(&avail, "en", tag("es"));
        assert_eq!(chosen.as_str(), "en-GB");
    }

    #[test]
    fn pick_respects_quality_order() {
        // de;q=1, fr;q=0.5. Both available. de wins.
        let avail = vec![tag("fr"), tag("de")];
        let chosen = pick_language(&avail, "fr;q=0.5, de;q=1.0", tag("en"));
        assert_eq!(chosen.as_str(), "de");
    }

    #[test]
    fn pick_wildcard_falls_through_to_first_available() {
        let avail = vec![tag("es"), tag("en")];
        let chosen = pick_language(&avail, "zh, *", tag("en"));
        // zh isn't available; * matches first in `available`.
        assert_eq!(chosen.as_str(), "es");
    }

    #[test]
    fn pick_falls_back_when_nothing_matches() {
        let avail = vec![tag("en"), tag("es")];
        let chosen = pick_language(&avail, "zh-TW, ja", tag("en"));
        assert_eq!(chosen.as_str(), "en");
    }

    #[test]
    fn pick_empty_header_returns_fallback() {
        let avail = vec![tag("en"), tag("es")];
        let chosen = pick_language(&avail, "", tag("en"));
        assert_eq!(chosen.as_str(), "en");
    }

    #[test]
    fn pick_garbage_header_returns_fallback() {
        let avail = vec![tag("en"), tag("es")];
        let chosen = pick_language(&avail, "!!!, en_US", tag("en"));
        assert_eq!(chosen.as_str(), "en");
    }

    #[test]
    fn pick_empty_available_returns_fallback() {
        let avail: Vec<LanguageTag> = vec![];
        let chosen = pick_language(&avail, "fr, de", tag("en"));
        assert_eq!(chosen.as_str(), "en");
    }

    #[test]
    fn pick_zh_tw_with_country_exact_match() {
        let avail = vec![tag("zh-TW"), tag("zh-CN"), tag("en")];
        let chosen = pick_language(&avail, "zh-TW", tag("en"));
        assert_eq!(chosen.as_str(), "zh-TW");
    }
}
