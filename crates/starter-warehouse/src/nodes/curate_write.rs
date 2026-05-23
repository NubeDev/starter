//! `starter.warehouse.curate-write` — typed write into `samples`.
//! Per-row Postgres entity lookup; W7 (never refuses payload
//! *structure*, but does refuse unknown `entity_id` for the
//! curated path — that's the documented difference vs `tap.write`).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::WarehouseRuntime;
use super::tap_write::{string_slot, tags_slot};
use crate::kinds::CURATE_WRITE;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    CURATE_WRITE,
    "starter.warehouse.curate-write.label",
    "starter.warehouse.curate-write.summary",
    "starter.warehouse.curate-write.help",
);

pub struct CurateWrite {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl CurateWrite {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self {
            rt,
            kind: KindId::new(CURATE_WRITE).unwrap(),
        }
    }
}

#[async_trait]
impl NodeBehavior for CurateWrite {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let entity_id = string_slot(&input, "entity_id")?;
        let ts: DateTime<Utc> = match input.get("ts") {
            Some(SlotValue::String(s)) => s
                .parse()
                .map_err(|e: chrono::ParseError| NodeError::InvalidInput(e.to_string()))?,
            _ => Utc::now(),
        };
        let value_num = match input.get("value_num") {
            Some(SlotValue::Float(f)) => Some(*f),
            Some(SlotValue::Int(i)) => Some(*i as f64),
            _ => None,
        };
        let tags = tags_slot(&input);
        self.rt
            .curate_write_sample(&entity_id, ts, value_num, tags)
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        Ok(SlotMap::new())
    }
}
