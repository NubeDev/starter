//! `starter.warehouse.mart-drop`. Required for AI experiment
//! cleanup. Moves catalog row to `quarantined`, drops the MV +
//! target table.

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::WarehouseRuntime;
use crate::kinds::MART_DROP;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    MART_DROP,
    "starter.warehouse.mart-drop.label",
    "starter.warehouse.mart-drop.summary",
    "starter.warehouse.mart-drop.help",
);

pub struct MartDrop {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl MartDrop {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self { rt, kind: KindId::new(MART_DROP).unwrap() }
    }
}

#[async_trait]
impl NodeBehavior for MartDrop {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let name = match input.get("name") {
            Some(SlotValue::String(s)) => s.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'name' slot".into())),
        };
        self.rt
            .mart_drop(&name)
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        Ok(SlotMap::new())
    }
}
