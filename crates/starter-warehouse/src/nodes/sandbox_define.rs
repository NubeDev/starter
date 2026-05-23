//! `starter.warehouse.sandbox-define`.

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::WarehouseRuntime;
use crate::ddl::sandbox::SandboxSpec;
use crate::kinds::SANDBOX_DEFINE;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    SANDBOX_DEFINE,
    "starter.warehouse.sandbox-define.label",
    "starter.warehouse.sandbox-define.summary",
    "starter.warehouse.sandbox-define.help",
);

pub struct SandboxDefine {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl SandboxDefine {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self {
            rt,
            kind: KindId::new(SANDBOX_DEFINE).unwrap(),
        }
    }
}

#[async_trait]
impl NodeBehavior for SandboxDefine {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let spec_v = match input.get("spec") {
            Some(SlotValue::Json(v)) => v.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'spec' slot".into())),
        };
        let owner = match input.get("owner") {
            Some(SlotValue::String(s)) => s.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'owner' slot".into())),
        };
        let spec: SandboxSpec = serde_json::from_value(spec_v.clone())
            .map_err(|e| NodeError::InvalidInput(e.to_string()))?;
        self.rt
            .sandbox_define(&owner, spec, spec_v)
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        Ok(SlotMap::new())
    }
}
