//! `starter.warehouse.mart-promote` (admin-gated). Inserts the
//! `ext_manifest_approvals` row for `ext:` marts as part of the
//! W12 trust seam.

use std::sync::Arc;

use async_trait::async_trait;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeDescriptor, NodeError, SlotMap, SlotValue,
};

use super::runtime::WarehouseRuntime;
use crate::kinds::MART_PROMOTE;

pub static DESCRIPTOR: NodeDescriptor = NodeDescriptor::new(
    MART_PROMOTE,
    "starter.warehouse.mart-promote.label",
    "starter.warehouse.mart-promote.summary",
    "starter.warehouse.mart-promote.help",
);

pub struct MartPromote {
    rt: Arc<WarehouseRuntime>,
    kind: KindId,
}

impl MartPromote {
    pub fn new(rt: Arc<WarehouseRuntime>) -> Self {
        Self {
            rt,
            kind: KindId::new(MART_PROMOTE).unwrap(),
        }
    }
}

#[async_trait]
impl NodeBehavior for MartPromote {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let name = match input.get("name") {
            Some(SlotValue::String(s)) => s.clone(),
            _ => return Err(NodeError::InvalidInput("missing 'name' slot".into())),
        };
        let approved_by = match input.get("approved_by") {
            Some(SlotValue::String(s)) => s.clone(),
            _ => "user:admin".to_string(),
        };
        let ext_manifest_hash = match input.get("ext_manifest_hash") {
            Some(SlotValue::String(s)) => Some(s.clone()),
            _ => None,
        };
        self.rt
            .mart_promote(&name, &approved_by, ext_manifest_hash.as_deref())
            .await
            .map_err(|e| NodeError::Backend(e.to_string()))?;
        Ok(SlotMap::new())
    }
}
