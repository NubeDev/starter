//! `starter.warehouse.sandbox-redefine`. RF-4: refused if frozen.

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::WarehouseRuntime;
use crate::kinds::SANDBOX_REDEFINE;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    SANDBOX_REDEFINE,
    "starter.warehouse.sandbox-redefine.label",
    "starter.warehouse.sandbox-redefine.summary",
    "starter.warehouse.sandbox-redefine.help",
);

pub struct SandboxRedefine {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl SandboxRedefine {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self { rt, kind: KindId::new(SANDBOX_REDEFINE).unwrap() }
    }
}

#[async_trait]
impl NodeBehavior for SandboxRedefine {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let name = match input.get("name") {
            Some(SlotValue::String(s)) => s.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'name' slot".into())),
        };
        let confirm = matches!(input.get("confirm"), Some(SlotValue::Bool(true)));
        let cols = match input.get("columns") {
            Some(SlotValue::Json(v)) => v.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'columns' slot".into())),
        };
        let rev = self
            .rt
            .sandbox_redefine(&name, confirm, cols)
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        let mut out = SlotMap::new();
        out.insert("columns_revision".into(), SlotValue::Int(rev));
        Ok(out)
    }
}
