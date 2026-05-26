//! `rubix.system.db` — tool dispatch.
//!
//! Stub probe: there is no DB pool wired yet. The dispatch shape
//! is locked here so the transport contract and the localisation
//! keys land in the same PR as the descriptor. See
//! [docs/design/migrations/](../../../../docs/design/migrations/README.md).

use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rubix_spi::dto::system::db::{DbHealthRequest, DbHealthResponse};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

/// Concrete `Tool` impl for `rubix.system.db`.
#[derive(Debug, Default)]
pub struct DbTool;

#[async_trait]
impl Tool for DbTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.system.db".to_owned(),
            description: "Report database engine reachability and engine-reported storage usage."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "dsn": {
                        "type": "string",
                        "description": "DSN override; absent means probe the booted DSN or an in-memory stub."
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: DbHealthRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("DbHealthRequest: {e}"),
        })?;
        let resp = probe(req)?;
        serde_json::to_value(resp).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

/// Pure dispatch — separated so integration tests can call it
/// without spinning up the MCP transport.
pub fn probe(req: DbHealthRequest) -> Result<DbHealthResponse> {
    let dsn = req.dsn.unwrap_or_else(|| "sqlite::memory:".to_owned());
    let probed_at_ms = now_epoch_ms();
    let used_bytes: u64 = 0;

    let code = MessageKey::parse("rubix.system.db.ok").expect("hard-coded key parses");
    let summary = Diagnostic::new(code)
        .with_param("used", DiagnosticParam::I64(used_bytes as i64))
        .with_param("at", DiagnosticParam::Timestamp(probed_at_ms));

    Ok(DbHealthResponse {
        summary,
        dsn,
        reachable: true,
        used_bytes,
        probed_at_ms,
    })
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_returns_ok_stub_with_default_dsn() {
        let resp = probe(DbHealthRequest::default()).expect("stub probe always succeeds");
        assert_eq!(resp.dsn, "sqlite::memory:");
        assert!(resp.reachable);
        assert_eq!(resp.summary.code.as_str(), "rubix.system.db.ok");
        assert!(resp.summary.params.contains_key("at"));
        assert!(resp.probed_at_ms > 0);
    }

    #[tokio::test]
    async fn probe_echoes_caller_supplied_dsn() {
        let resp = probe(DbHealthRequest {
            dsn: Some("postgres://example/local".to_owned()),
        })
        .expect("stub probe always succeeds");
        assert_eq!(resp.dsn, "postgres://example/local");
    }
}
