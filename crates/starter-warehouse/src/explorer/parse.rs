//! Read-only SQL classifier for `POST /api/warehouse/ch/query`.
//!
//! The plan deliberately avoids a SQL parser dependency
//! (`sqlparser` is not in the workspace and is not getting in for
//! this work). Instead we do a cheap, case-insensitive leading-
//! token check after stripping line / block comments and leading
//! whitespace. ClickHouse server-side `SETTINGS readonly = 2` is
//! the second line of defence (applied at the HTTP transport in
//! `crate::explorer::queries::query`).
//!
//! Accepted leading tokens:
//!
//! - `SELECT …`
//! - `WITH … SELECT …` (the trailing `SELECT` is **not** verified
//!   here — `readonly = 2` handles the case where someone smuggles
//!   `WITH … INSERT …`; we keep the classifier simple).
//! - `SHOW …`
//! - `DESCRIBE` / `DESC`
//! - `EXPLAIN …`
//!
//! Everything else is rejected with [`Reject::NotReadOnly`].

/// Result of classifying a candidate SQL statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Statement may be sent to ClickHouse.
    Allow,
    /// Statement must not leave the server. The `&'static str`
    /// is a short machine-readable reason suitable for the
    /// `{"error": ...}` response body.
    Reject(Reject),
}

/// Why a statement was rejected. Kept narrow so the
/// `POST /query` handler can map directly onto HTTP responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    /// The statement is empty (or just whitespace / comments).
    Empty,
    /// The leading token is not in the read-only allow-list.
    NotReadOnly,
}

impl Reject {
    pub fn as_str(self) -> &'static str {
        match self {
            Reject::Empty => "empty_query",
            Reject::NotReadOnly => "read_only_violation",
        }
    }
}

/// Classify `sql`. See module docs for the accepted set.
pub fn classify(sql: &str) -> Verdict {
    let trimmed = strip_leading_noise(sql);
    if trimmed.is_empty() {
        return Verdict::Reject(Reject::Empty);
    }
    let leading = leading_word(trimmed).to_ascii_uppercase();
    match leading.as_str() {
        "SELECT" | "WITH" | "SHOW" | "DESCRIBE" | "DESC" | "EXPLAIN" => Verdict::Allow,
        _ => Verdict::Reject(Reject::NotReadOnly),
    }
}

/// Strip leading whitespace, `--`-to-EOL comments, and `/* … */`
/// block comments. Returns the remaining slice.
fn strip_leading_noise(mut s: &str) -> &str {
    loop {
        let before = s.len();
        s = s.trim_start();
        if let Some(rest) = s.strip_prefix("--") {
            // Line comment: drop through to end-of-line.
            s = rest.split_once('\n').map(|x| x.1).unwrap_or("");
        } else if let Some(rest) = s.strip_prefix("/*") {
            // Block comment: drop through matching `*/`. If
            // unterminated, treat the rest as comment.
            s = match rest.find("*/") {
                Some(end) => &rest[end + 2..],
                None => "",
            };
        }
        if s.len() == before {
            return s;
        }
    }
}

/// Return the leading alphabetic / underscore word from `s`,
/// stopping at the first non-word byte.
fn leading_word(s: &str) -> &str {
    let end = s
        .as_bytes()
        .iter()
        .position(|b| !b.is_ascii_alphabetic() && *b != b'_')
        .unwrap_or(s.len());
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow(sql: &str) {
        assert_eq!(classify(sql), Verdict::Allow, "expected Allow: {sql:?}");
    }

    fn reject_not_ro(sql: &str) {
        assert_eq!(
            classify(sql),
            Verdict::Reject(Reject::NotReadOnly),
            "expected NotReadOnly reject: {sql:?}",
        );
    }

    #[test]
    fn accepts_basic_read_shapes() {
        allow("SELECT 1");
        allow("select 1");
        allow("  SELECT 1");
        allow("\n\tSELECT 1");
        allow("WITH cte AS (SELECT 1) SELECT * FROM cte");
        allow("SHOW TABLES");
        allow("SHOW DATABASES");
        allow("SHOW CREATE TABLE samples");
        allow("DESCRIBE samples");
        allow("DESC samples");
        allow("EXPLAIN SELECT 1");
    }

    #[test]
    fn accepts_with_leading_comments() {
        allow("-- comment\nSELECT 1");
        allow("/* block */ SELECT 1");
        allow("/* multi\nline */ SELECT 1");
        allow("-- one\n-- two\n  SELECT 1");
    }

    #[test]
    fn unterminated_block_comment_classifies_as_empty() {
        assert_eq!(classify("/* unterminated"), Verdict::Reject(Reject::Empty));
    }

    #[test]
    fn rejects_every_write_verb_we_care_about() {
        for v in [
            "INSERT INTO foo VALUES (1)",
            "ALTER TABLE foo DROP COLUMN x",
            "OPTIMIZE TABLE foo",
            "TRUNCATE TABLE foo",
            "KILL QUERY WHERE 1",
            "SYSTEM RELOAD DICTIONARY foo",
            "CREATE TABLE foo (id UInt64) ENGINE = MergeTree ORDER BY id",
            "DROP TABLE foo",
            "RENAME TABLE foo TO bar",
            "ATTACH TABLE foo",
            "DETACH TABLE foo",
            "SET max_memory_usage = 1",
            "USE other_db",
        ] {
            reject_not_ro(v);
        }
    }

    #[test]
    fn rejects_empty_and_whitespace_only() {
        assert_eq!(classify(""), Verdict::Reject(Reject::Empty));
        assert_eq!(classify("   \n\t  "), Verdict::Reject(Reject::Empty));
        assert_eq!(
            classify("-- just a comment"),
            Verdict::Reject(Reject::Empty)
        );
        assert_eq!(classify("/* only block */"), Verdict::Reject(Reject::Empty));
    }

    #[test]
    fn rejects_nonsense_leading_token() {
        reject_not_ro("FOO bar");
        reject_not_ro(";SELECT 1");
        reject_not_ro("(SELECT 1)");
    }
}
