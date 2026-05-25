//! Raw HTTP escape hatch for dynamic-shape SELECTs.
//!
//! The typed `clickhouse` crate is excellent for `Row`-derived
//! tables (`raw_events`, `samples`, `events`, `documents`, …) but
//! has no first-class way to fetch a result set whose schema is
//! only known at runtime — which is precisely what the explorer's
//! `POST /query` and `table_data` paths need.
//!
//! Rather than poke a `Row`-typed hole through the typed surface,
//! this module exposes [`ChClient::fetch_json`]: a thin `reqwest`
//! POST that runs `<sql> FORMAT JSONCompactEachRow` under
//! `SETTINGS readonly = 2`, parses the streamed JSON envelope, and
//! returns a `{columns, rows}` structure the explorer hands
//! straight to the UI.
//!
//! The W8 invariant ("no raw `INSERT INTO (raw_events|samples|
//! events|documents)` outside `src/store/`") is preserved two
//! ways:
//!
//!  1. We reject any statement whose leading token is in
//!     [`WRITE_VERBS`] before the request leaves the process.
//!     This is the cheap, in-process belt half of belt-and-braces
//!     (mirrors the explorer's `parse::classify` allow-list
//!     without depending on it — that one lives in the consumer
//!     crate).
//!  2. We pin `readonly=2` server-side; even if a write verb
//!     slipped through the leading-token check, ClickHouse would
//!     refuse it.
//!
//! Forking note: the `forbid_raw_insert` doctest is the project's
//! grep-level invariant that nothing outside `src/store/` writes
//! `INSERT INTO (raw_events|samples|events|documents)` as a raw
//! string. This module never produces such a string — it forwards
//! caller-supplied SQL, and the leading-token gate refuses
//! `INSERT` outright.

use serde::Serialize;

use crate::client::{ChClient, ChClientError};

/// Verbs we refuse to forward to ClickHouse. The list is the same
/// shape as the explorer's `parse` allow-list (inverted) and is
/// intentionally narrow — `readonly=2` on the server is the
/// authoritative gate.
const WRITE_VERBS: &[&str] = &[
    "INSERT", "ALTER", "OPTIMIZE", "TRUNCATE", "KILL", "SYSTEM", "CREATE", "DROP", "RENAME",
    "ATTACH", "DETACH", "GRANT", "REVOKE", "SET", "USE", "UPDATE", "DELETE", "REPLACE", "EXCHANGE",
];

/// Columns + rows envelope returned by [`ChClient::fetch_json`].
/// Matches the shape sql-studio's UI consumes (`{ columns, rows }`)
/// so the explorer's `TableData` / `Query` responses can wrap it
/// without further massaging.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

impl ChClient {
    /// Run `sql` over the raw HTTP transport, expecting a result
    /// set, and parse it as JSON. Always wraps the request in
    /// `SETTINGS readonly = 2`. Rejects statements whose leading
    /// token is in [`WRITE_VERBS`] before hitting the wire.
    ///
    /// The query is sent with the explicit `FORMAT JSONCompactEachRow`
    /// suffix and we pass `default_format=JSONCompact` so the
    /// envelope CH returns is `{ "meta": [...], "data": [...] }`
    /// — that gives us both the columns and the rows in one
    /// round-trip.
    ///
    /// ```no_run
    /// # use starter_store_clickhouse::{ChClient, ChConfig};
    /// # async fn demo(client: ChClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = client.fetch_json("SELECT 1 AS one").await?;
    /// assert_eq!(rows.columns, vec!["one".to_string()]);
    /// # Ok(()) }
    /// ```
    ///
    /// Statements starting with a write verb are rejected before
    /// the request is sent — preserving the spirit of the
    /// `forbid_raw_insert` rule even though the writes would not
    /// pass `readonly=2` server-side either:
    ///
    /// ```
    /// # use starter_store_clickhouse::{ChClient, ChConfig};
    /// # tokio_test::block_on(async {
    /// let client = ChClient::connect(ChConfig::local("http://127.0.0.1:1"));
    /// let err = client.fetch_json("INSERT INTO samples VALUES (1)").await.unwrap_err();
    /// assert!(err.to_string().contains("read-only"));
    /// # });
    /// ```
    pub async fn fetch_json(&self, sql: &str) -> Result<JsonRows, ChClientError> {
        if let Some(verb) = leading_write_verb(sql) {
            return Err(ChClientError::Other(format!(
                "read-only HTTP path refused leading verb {verb}",
            )));
        }

        let cfg = self.config();
        // Compose the URL. Default-format gives us `{meta, data}`;
        // database/user are query-string params per the CH HTTP
        // wire docs.
        let mut url = reqwest::Url::parse(&cfg.url).map_err(|e| ChClientError::Other(e.to_string()))?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("database", &cfg.database);
            q.append_pair("default_format", "JSONCompact");
            q.append_pair("readonly", "2");
        }

        let body = sql.to_string();
        let resp = reqwest::Client::new()
            .post(url)
            .basic_auth(&cfg.user, Some(&cfg.password))
            .body(body)
            .send()
            .await
            .map_err(|e| ChClientError::Other(format!("clickhouse http: {e}")))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| ChClientError::Other(format!("clickhouse http body: {e}")))?;
        if !status.is_success() {
            return Err(ChClientError::Other(format!(
                "clickhouse http {}: {}",
                status.as_u16(),
                text.trim()
            )));
        }

        // Empty body (some DDL-shaped queries) -> empty envelope.
        if text.trim().is_empty() {
            return Ok(JsonRows {
                columns: Vec::new(),
                rows: Vec::new(),
            });
        }

        let envelope: JsonCompactEnvelope = serde_json::from_str(&text)?;
        let columns = envelope
            .meta
            .into_iter()
            .map(|m| m.name)
            .collect::<Vec<_>>();
        let rows = envelope.data;
        Ok(JsonRows { columns, rows })
    }
}

#[derive(serde::Deserialize)]
struct JsonCompactEnvelope {
    #[serde(default)]
    meta: Vec<JsonCompactMeta>,
    #[serde(default)]
    data: Vec<Vec<serde_json::Value>>,
}

#[derive(serde::Deserialize)]
struct JsonCompactMeta {
    name: String,
    // `type` is present in the wire payload but the explorer
    // doesn't surface it today; drop on parse.
    #[serde(rename = "type", default)]
    _ty: serde_json::Value,
}

fn leading_write_verb(sql: &str) -> Option<&'static str> {
    let trimmed = strip_leading_noise(sql);
    let word: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_alphabetic() || *c == '_')
        .collect::<String>()
        .to_ascii_uppercase();
    WRITE_VERBS.iter().copied().find(|v| *v == word)
}

fn strip_leading_noise(mut s: &str) -> &str {
    loop {
        let before = s.len();
        s = s.trim_start();
        if let Some(rest) = s.strip_prefix("--") {
            s = rest.splitn(2, '\n').nth(1).unwrap_or("");
        } else if let Some(rest) = s.strip_prefix("/*") {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_write_verb_recognises_common_writes() {
        assert_eq!(leading_write_verb("INSERT INTO foo VALUES (1)"), Some("INSERT"));
        assert_eq!(leading_write_verb("alter table foo"), Some("ALTER"));
        assert_eq!(
            leading_write_verb("  -- preface\n  DROP TABLE foo"),
            Some("DROP"),
        );
        assert_eq!(leading_write_verb("/* x */ TRUNCATE TABLE foo"), Some("TRUNCATE"));
    }

    #[test]
    fn leading_write_verb_passes_through_reads() {
        assert_eq!(leading_write_verb("SELECT 1"), None);
        assert_eq!(leading_write_verb("with cte AS (select 1) select * from cte"), None);
        assert_eq!(leading_write_verb("SHOW TABLES"), None);
        assert_eq!(leading_write_verb(""), None);
    }
}
