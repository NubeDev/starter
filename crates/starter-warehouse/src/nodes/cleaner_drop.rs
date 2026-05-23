//! `starter.warehouse.cleaner-drop`. Clears `frozen_at_revision`
//! on the source sandbox per RF-4 / RF-6.

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::WarehouseRuntime;
use crate::kinds::CLEANER_DROP;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    CLEANER_DROP,
    "starter.warehouse.cleaner-drop.label",
    "starter.warehouse.cleaner-drop.summary",
    "starter.warehouse.cleaner-drop.help",
);

pub struct CleanerDrop {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl CleanerDrop {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self { rt, kind: KindId::new(CLEANER_DROP).unwrap() }
    }
}

#[async_trait]
impl NodeBehavior for CleanerDrop {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let name = match input.get("name") {
            Some(SlotValue::String(s)) => s.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'name' slot".into())),
        };
        self.rt
            .cleaner_drop(&name)
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        Ok(SlotMap::new())
    }
}
