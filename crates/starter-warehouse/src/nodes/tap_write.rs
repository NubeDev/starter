//! `starter.warehouse.tap-write` — W7 / W8. One row into
//! `raw_events`; never refuses payload structure.
//!
//! Input slots:
//! - `source` String
//! - `payload` String (raw JSON text)
//! - `tags` Json (object of string→string)
//!
//! Output slots:
//! - `event_id` Int (u64 truncated)

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::WarehouseRuntime;
use crate::kinds::TAP_WRITE;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    TAP_WRITE,
    "starter.warehouse.tap-write.label",
    "starter.warehouse.tap-write.summary",
    "starter.warehouse.tap-write.help",
);

pub struct TapWrite {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl TapWrite {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self {
            rt,
            kind: KindId::new(TAP_WRITE).expect("valid kind id"),
        }
    }
}

#[async_trait]
impl NodeBehavior for TapWrite {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(
        &self,
        _ctx: NodeCtx<'_>,
        input: SlotMap,
    ) -> Result<SlotMap, NodeError> {
        let source = string_slot(&input, "source")?;
        let payload = string_slot(&input, "payload")?;
        let tags = tags_slot(&input);
        // W7 — never refuses. Empty payload → empty raw_events row
        // with a `quality` flag. A malformed JSON payload still
        // lands; downstream curation classifies.
        let id = self
            .rt
            .tap_write(&source, payload, tags)
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        let mut out = SlotMap::new();
        out.insert("event_id".into(), SlotValue::Int(id as i64));
        Ok(out)
    }
}

pub(crate) fn string_slot(m: &SlotMap, k: &str) -> Result<String, NodeError> {
    match m.get(k) {
        Some(SlotValue::String(s)) => Ok(s.clone()),
        Some(_) => Err(NodeError::InvalidInput(format!("slot {k:?} must be String"))),
        None => Err(NodeError::InvalidInput(format!("missing slot {k:?}"))),
    }
}

pub(crate) fn tags_slot(m: &SlotMap) -> Vec<(String, String)> {
    match m.get("tags") {
        Some(SlotValue::Json(serde_json::Value::Object(o))) => o
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect(),
        _ => Vec::new(),
    }
}
