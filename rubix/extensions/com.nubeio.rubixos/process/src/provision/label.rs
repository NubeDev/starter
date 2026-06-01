//! `bc_label_render` — device → printable label payload.
//!
//! Closes the loop for devices that arrive without a Nube-iO sticker
//! (manual add): render the canonical QR URL + Code128 fallback +
//! human serial so the device can be re-scanned later (BARCODE.md §2).
//!
//! This is a *catalog* read + format: it reads the device row through
//! the read template and re-emits its identity in both barcode
//! grammars. It mints no new state.

use starter_ext_sdk::serde_json::{json, Value};
use starter_ext_sdk::Error;

use crate::provision::crud::take_str;
use crate::provision::RubixOsCtx;

/// Render the printable label for one device id.
pub fn handle(ctx: &RubixOsCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    let device_id = take_str(params, "device_id", "bc_label_render")?;

    let rows = ctx.warehouse_read().query(
        "com.nubeio.rubixos.bc_devices_list",
        json!({ "limit": 500 }),
    )?;
    let device = rows
        .into_iter()
        .map(|r| Value::Object(r.0))
        .find(|r| r.get("device_id").and_then(Value::as_str) == Some(device_id.as_str()))
        .ok_or_else(|| {
            Error::Validation(format!("bc_label_render: device `{device_id}` not found"))
        })?;

    let model = device
        .get("template")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let network = device
        .get("network")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let address = device.get("address").and_then(Value::as_str);
    let display_name = device.get("name").and_then(Value::as_str);

    let qr_url = qr_url(&device_id, model, network, address);
    let code128 = code128(&device_id, model, network, address);

    Ok(json!({
        "device_id": device_id,
        "serial": device_id,
        "qr_url": qr_url,
        "code128": code128,
        "display_name": display_name,
    }))
}

/// Canonical `rubix://add?…` URL form (BARCODE.md §2).
fn qr_url(id: &str, model: &str, network: &str, address: Option<&str>) -> String {
    let mut url = format!("rubix://add?v=1&id={id}&model={model}&network={network}");
    if let Some(addr) = address.filter(|a| !a.is_empty()) {
        let slot = if network == "bacnet" { "addr" } else { "eui" };
        url.push_str(&format!("&{slot}={addr}"));
    }
    url
}

/// Code128 pipe-delimited fallback (BARCODE.md §2).
fn code128(id: &str, model: &str, network: &str, address: Option<&str>) -> String {
    format!("1|{id}|{model}|{network}|{}", address.unwrap_or(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_lora_qr_and_code128() {
        assert_eq!(
            qr_url("DRP-1", "droplet", "lora", Some("70B3D5")),
            "rubix://add?v=1&id=DRP-1&model=droplet&network=lora&eui=70B3D5"
        );
        assert_eq!(
            code128("DRP-1", "droplet", "lora", Some("70B3D5")),
            "1|DRP-1|droplet|lora|70B3D5"
        );
    }

    #[test]
    fn bacnet_uses_addr_slot() {
        assert!(qr_url("B-1", "io_22", "bacnet", Some("12")).contains("&addr=12"));
    }

    #[test]
    fn omits_empty_address() {
        assert_eq!(
            qr_url("X", "m", "rubix", None),
            "rubix://add?v=1&id=X&model=m&network=rubix"
        );
    }
}
