//! `starter.warehouse.cleaner-promote` (admin-gated).

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::WarehouseRuntime;
use crate::kinds::CLEANER_PROMOTE;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    CLEANER_PROMOTE,
    "starter.warehouse.cleaner-promote.label",
    "starter.warehouse.cleaner-promote.summary",
    "starter.warehouse.cleaner-promote.help",
);

pub struct CleanerPromote {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl CleanerPromote {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self { rt, kind: KindId::new(CLEANER_PROMOTE).unwrap() }
    }
}

#[async_trait]
impl NodeBehavior for CleanerPromote {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let name = match input.get("name") {
            Some(SlotValue::String(s)) => s.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'name' slot".into())),
        };
        self.rt
            .cleaner_promote(&name)
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        Ok(SlotMap::new())
    }
}
