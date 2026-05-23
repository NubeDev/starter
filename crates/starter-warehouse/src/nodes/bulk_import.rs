//! `starter.warehouse.bulk-import` — W8a. The only sanctioned
//! non-`async_insert=1` path. `target` is REQUIRED; there is no
//! default. Batches of 10k rows are flushed via `async_insert=0`.

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::{BulkTarget, WarehouseRuntime};
use crate::kinds::BULK_IMPORT;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    BULK_IMPORT,
    "starter.warehouse.bulk-import.label",
    "starter.warehouse.bulk-import.summary",
    "starter.warehouse.bulk-import.help",
);

pub struct BulkImport {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl BulkImport {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self {
            rt,
            kind: KindId::new(BULK_IMPORT).unwrap(),
        }
    }
}

#[async_trait]
impl NodeBehavior for BulkImport {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let target = match input.get("target") {
            Some(SlotValue::String(s)) => parse_target(s)?,
            None => {
                return Err(NodeError::InvalidInput(
                    "bulk.import requires a target ('samples' | 'sandbox:<name>' | 'raw_events') — no default"
                        .into(),
                ))
            }
            _ => {
                return Err(NodeError::InvalidInput(
                    "bulk.import target must be a String".into(),
                ))
            }
        };
        let rows = match input.get("rows") {
            Some(SlotValue::Json(v)) => serde_json::from_value(v.clone())
                .map_err(|e| NodeError::InvalidInput(e.to_string()))?,
            _ => Vec::new(),
        };
        let total = self
            .rt
            .bulk_import_samples(target, rows)
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        let mut out = SlotMap::new();
        out.insert("rows_written".into(), SlotValue::Int(total as i64));
        Ok(out)
    }
}

fn parse_target(s: &str) -> Result<BulkTarget, NodeError> {
    match s {
        "samples" => Ok(BulkTarget::Samples),
        "raw_events" => Ok(BulkTarget::RawEvents),
        other if other.starts_with("sandbox:") => {
            Ok(BulkTarget::Sandbox(other["sandbox:".len()..].to_string()))
        }
        other => Err(NodeError::InvalidInput(format!(
            "unknown bulk.import target {other:?}"
        ))),
    }
}
