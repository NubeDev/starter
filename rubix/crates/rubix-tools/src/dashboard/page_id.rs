//! Shared `page_id` validation for the `rubix.dashboard.*` write
//! verbs (`create`, `duplicate`, …).
//!
//! Background: stored page ids carry a `dashboard.<slug>` prefix
//! that distinguishes SDUI page rows from other resource ids that
//! may share the table or grammar later. The URL form at
//! `/dashboards/<slug>` strips it. This double-form trips up
//! consumers — callers pass the URL-form bare slug to an API that
//! wants the stored form, or vice versa, and the previous generic
//! "must match dashboard.<lowercase-slug>" error didn't help them
//! see the fix.
//!
//! [`validate_stored_page_id`] returns a structured
//! [`PageIdError`] so the verbs can hand back a diagnostic that
//! names the specific failure mode (bare slug, wrong prefix,
//! illegal char, too long) and, where the input is obviously a
//! bare slug, suggests the corrected stored form. Issue #5 of
//! `rubix/docs/design/sdui/dashboard-api-usage.md`.

/// Why a candidate page id failed [`validate_stored_page_id`].
#[derive(Debug, PartialEq, Eq)]
pub enum PageIdError<'a> {
    /// The id has no `dashboard.` prefix but otherwise looks like
    /// a valid slug — almost certainly the URL form. Carries the
    /// suggested stored form so the verb can echo it back to the
    /// caller.
    BareSlug {
        /// The slug as received.
        slug: &'a str,
        /// The corrected stored form (`"dashboard.<slug>"`).
        suggestion: String,
    },
    /// The id has no `dashboard.` prefix and the leading segment
    /// doesn't look like a slug either — the caller passed
    /// something unrelated (empty, wrong namespace, etc.).
    WrongPrefix(&'a str),
    /// The id is `dashboard.` with an empty slug.
    EmptySlug,
    /// The slug exceeds the 128-char cap.
    SlugTooLong {
        /// Length of the slug portion.
        len: usize,
    },
    /// The slug contains a character outside the allowed grammar
    /// (`[a-z0-9-.]`).
    IllegalChar {
        /// The offending character.
        ch: char,
    },
}

impl PageIdError<'_> {
    /// Human-readable explanation suitable for an `Error::Invalid`
    /// message. Includes a corrected example when the failure mode
    /// is the bare-slug confusion documented as issue #5.
    pub fn message(&self, field: &str) -> String {
        match self {
            Self::BareSlug { slug, suggestion } => format!(
                "{field} `{slug}` looks like the URL form — pass the stored \
                 form `{suggestion}` instead. SDUI page ids are stored as \
                 `dashboard.<slug>`; the URL at `/dashboards/<slug>` strips \
                 the prefix."
            ),
            Self::WrongPrefix(got) => format!(
                "{field} `{got}` must match `dashboard.<lowercase-slug>` \
                 (e.g. `dashboard.ops`)."
            ),
            Self::EmptySlug => format!(
                "{field} `dashboard.` has an empty slug — expected \
                 `dashboard.<lowercase-slug>`."
            ),
            Self::SlugTooLong { len } => format!(
                "{field} slug is {len} chars; the cap is 128."
            ),
            Self::IllegalChar { ch } => format!(
                "{field} slug contains `{ch}`; only [a-z0-9-.] are allowed."
            ),
        }
    }
}

/// Validate a candidate stored-form page id.
///
/// Returns `Ok(())` on success or a [`PageIdError`] that the
/// caller turns into a user-facing diagnostic via
/// [`PageIdError::message`].
pub fn validate_stored_page_id(id: &str) -> std::result::Result<(), PageIdError<'_>> {
    let Some(slug) = id.strip_prefix("dashboard.") else {
        // Try to recognise the bare-slug case so the caller gets a
        // concrete fix. "Looks like a slug" ⇔ non-empty, in-grammar,
        // not already prefixed (the `strip_prefix` above failed),
        // and not containing the prefix mid-string.
        if !id.is_empty() && id.len() <= 128 && is_slug_grammar(id) {
            return Err(PageIdError::BareSlug {
                slug: id,
                suggestion: format!("dashboard.{id}"),
            });
        }
        return Err(PageIdError::WrongPrefix(id));
    };
    if slug.is_empty() {
        return Err(PageIdError::EmptySlug);
    }
    if slug.len() > 128 {
        return Err(PageIdError::SlugTooLong { len: slug.len() });
    }
    if let Some(ch) = slug.chars().find(|c| !is_slug_char(*c)) {
        return Err(PageIdError::IllegalChar { ch });
    }
    Ok(())
}

fn is_slug_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.'
}

fn is_slug_grammar(s: &str) -> bool {
    s.chars().all(is_slug_char)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_form_is_accepted() {
        assert!(validate_stored_page_id("dashboard.ops").is_ok());
        assert!(validate_stored_page_id("dashboard.ops-v2").is_ok());
        assert!(validate_stored_page_id("dashboard.sub.area.42").is_ok());
    }

    #[test]
    fn bare_slug_suggests_stored_form() {
        let err = validate_stored_page_id("ops").unwrap_err();
        match err {
            PageIdError::BareSlug { slug, suggestion } => {
                assert_eq!(slug, "ops");
                assert_eq!(suggestion, "dashboard.ops");
            }
            other => panic!("expected BareSlug, got {other:?}"),
        }
        // The diagnostic message mentions both forms so the caller
        // can act without re-reading the docs.
        let msg = validate_stored_page_id("ops").unwrap_err().message("page_id");
        assert!(msg.contains("dashboard.ops"));
        assert!(msg.contains("URL"));
    }

    #[test]
    fn wrong_prefix_is_rejected_without_suggestion() {
        // Empty string takes the WrongPrefix branch (BareSlug
        // requires non-empty).
        assert!(matches!(
            validate_stored_page_id("").unwrap_err(),
            PageIdError::WrongPrefix("")
        ));
        // Uppercase falls outside slug grammar so BareSlug doesn't
        // fire either; the caller is told the prefix is wrong.
        assert!(matches!(
            validate_stored_page_id("Dashboard.Ops").unwrap_err(),
            PageIdError::WrongPrefix(_)
        ));
    }

    #[test]
    fn empty_slug_after_prefix_is_flagged() {
        assert_eq!(
            validate_stored_page_id("dashboard.").unwrap_err(),
            PageIdError::EmptySlug
        );
    }

    #[test]
    fn slug_too_long_is_flagged_with_length() {
        let long = format!("dashboard.{}", "a".repeat(129));
        match validate_stored_page_id(&long).unwrap_err() {
            PageIdError::SlugTooLong { len } => assert_eq!(len, 129),
            other => panic!("expected SlugTooLong, got {other:?}"),
        }
    }

    #[test]
    fn illegal_char_in_slug_is_flagged() {
        match validate_stored_page_id("dashboard.Ops").unwrap_err() {
            PageIdError::IllegalChar { ch } => assert_eq!(ch, 'O'),
            other => panic!("expected IllegalChar, got {other:?}"),
        }
        match validate_stored_page_id("dashboard.ops!").unwrap_err() {
            PageIdError::IllegalChar { ch } => assert_eq!(ch, '!'),
            other => panic!("expected IllegalChar, got {other:?}"),
        }
    }
}
