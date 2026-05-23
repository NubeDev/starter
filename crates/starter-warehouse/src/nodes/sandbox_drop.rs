//! `starter.warehouse.sandbox-drop`.

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::WarehouseRuntime;
use crate::kinds::SANDBOX_DROP;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    SANDBOX_DROP,
    "starter.warehouse.sandbox-drop.label",
    "starter.warehouse.sandbox-drop.summary",
    "starter.warehouse.sandbox-drop.help",
);

pub struct SandboxDrop {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl SandboxDrop {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self { rt, kind: KindId::new(SANDBOX_DROP).unwrap() }
    }
}

#[async_trait]
impl NodeBehavior for SandboxDrop {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let name = match input.get("name") {
            Some(SlotValue::String(s)) => s.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'name' slot".into())),
        };
        self.rt
            .sandbox_drop(&name)
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        Ok(SlotMap::new())
    }
}
