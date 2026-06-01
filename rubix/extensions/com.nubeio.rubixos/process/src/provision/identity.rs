//! The normalised device identity decoded from a barcode.
//!
//! Both the canonical QR form
//! (`rubix://add?v=1&id=DRP-9F2C18&model=droplet&network=lora&eui=…`)
//! and the Code128 fallback (`1|DRP-9F2C18|droplet|lora|70B3D5…`)
//! decode into the same [`ScannedIdentity`]. Downstream provisioning
//! never sees the wire form — only this struct.

use starter_ext_sdk::serde_json::{json, Value};

/// Identity carried on a device sticker, after normalisation.
///
/// The barcode carries the *minimum* identity needed to look the
/// device up in a template and reach it on the network. Secrets and
/// full config live in the template, never on the sticker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedIdentity {
    /// Globally-unique device id (also the human-readable serial).
    pub id: String,
    /// Template lookup key — matched against `bc_templates.template`.
    pub model: String,
    /// Transport: `lora` | `bacnet` | `rubix`.
    pub network: String,
    /// On-air address: LoRa DevEUI or BACnet MAC. `None` when absent.
    pub address: Option<String>,
    /// Management IP for `rubix`/`bacnet`; `None` for `lora`.
    pub default_ip: Option<String>,
    /// Hardware revision the template may branch on.
    pub hw: Option<String>,
}

impl ScannedIdentity {
    /// Shape the identity for the `bc_decode` tool response, merging
    /// in the resolved template summary and the list of known models.
    pub fn to_decode_output(&self, template: Value, known_models: Vec<String>) -> Value {
        json!({
            "id": self.id,
            "model": self.model,
            "network": self.network,
            "address": self.address,
            "default_ip": self.default_ip,
            "hw": self.hw,
            "template": template,
            "known_models": known_models,
        })
    }
}
