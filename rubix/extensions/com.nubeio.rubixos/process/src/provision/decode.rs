//! Barcode string → [`ScannedIdentity`]. Pure, no DB access.
//!
//! Accepts the two on-sticker grammars defined in BARCODE.md §2:
//!
//!  - canonical QR: `rubix://add?v=1&id=DRP-9F2C18&model=droplet&network=lora&eui=70B3D5…`
//!  - Code128 fallback: `1|DRP-9F2C18|droplet|lora|70B3D5…`
//!
//! Both normalise to the same struct. A malformed payload is a
//! `Validation` error with a human message — never a panic.

use starter_ext_sdk::Error;

use crate::provision::identity::ScannedIdentity;

/// Decode a raw barcode string into a normalised identity.
pub fn decode(barcode: &str) -> starter_ext_sdk::Result<ScannedIdentity> {
    let trimmed = barcode.trim();
    if trimmed.is_empty() {
        return Err(Error::Validation("bc_decode: `barcode` is empty".into()));
    }
    if let Some(rest) = trimmed.strip_prefix("rubix://add?") {
        decode_qr(rest)
    } else if trimmed.contains('|') {
        decode_code128(trimmed)
    } else {
        Err(Error::Validation(format!(
            "bc_decode: unrecognised barcode `{trimmed}` — expected a \
             `rubix://add?…` URL or a `v|id|model|network|addr` Code128 payload"
        )))
    }
}

/// Parse the query-string body of a `rubix://add?…` URL.
fn decode_qr(query: &str) -> starter_ext_sdk::Result<ScannedIdentity> {
    let mut id = None;
    let mut model = None;
    let mut network = None;
    let mut address = None;
    let mut default_ip = None;
    let mut hw = None;

    for pair in query.split('&') {
        let Some((key, raw)) = pair.split_once('=') else {
            continue;
        };
        let value = percent_decode(raw);
        match key {
            "id" => id = non_empty(value),
            "model" => model = non_empty(value),
            "network" => network = non_empty(value),
            // `eui` (LoRa) and `addr` (BACnet) are the same slot.
            "eui" | "addr" => address = non_empty(value),
            "default_ip" | "ip" => default_ip = non_empty(value),
            "hw" => hw = non_empty(value),
            // `v` (schema version) is accepted and ignored for v=1.
            _ => {}
        }
    }

    build(id, model, network, address, default_ip, hw)
}

/// Parse the pipe-delimited Code128 fallback:
/// `v | id | model | network | addr`.
fn decode_code128(payload: &str) -> starter_ext_sdk::Result<ScannedIdentity> {
    let fields: Vec<&str> = payload.split('|').collect();
    if fields.len() < 4 {
        return Err(Error::Validation(format!(
            "bc_decode: Code128 payload `{payload}` has {} fields; \
             expected at least `v|id|model|network`",
            fields.len()
        )));
    }
    let id = non_empty(fields[1].to_owned());
    let model = non_empty(fields[2].to_owned());
    let network = non_empty(fields[3].to_owned());
    let address = fields.get(4).and_then(|s| non_empty((*s).to_owned()));
    build(id, model, network, address, None, None)
}

/// Assemble the identity, enforcing the three required fields.
fn build(
    id: Option<String>,
    model: Option<String>,
    network: Option<String>,
    address: Option<String>,
    default_ip: Option<String>,
    hw: Option<String>,
) -> starter_ext_sdk::Result<ScannedIdentity> {
    let id = id.ok_or_else(|| Error::Validation("bc_decode: missing `id`".into()))?;
    let model = model.ok_or_else(|| Error::Validation("bc_decode: missing `model`".into()))?;
    let network =
        network.ok_or_else(|| Error::Validation("bc_decode: missing `network`".into()))?;
    Ok(ScannedIdentity {
        id,
        model,
        network,
        address,
        default_ip,
        hw,
    })
}

/// Map an empty string to `None` so absent and blank fields behave
/// identically downstream.
fn non_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Minimal `%XX` and `+` percent-decoding for barcode query values.
///
/// Barcodes carry simple ASCII identities (ids, model keys, hex
/// EUIs); a dependency-free decoder keeps the SCOPE R8 single-dep
/// rule intact. Invalid escapes are passed through verbatim.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi * 16 + lo) as u8);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_canonical_qr() {
        let id = decode("rubix://add?v=1&id=DRP-9F2C18&model=droplet&network=lora&eui=70B3D5")
            .unwrap();
        assert_eq!(id.id, "DRP-9F2C18");
        assert_eq!(id.model, "droplet");
        assert_eq!(id.network, "lora");
        assert_eq!(id.address.as_deref(), Some("70B3D5"));
        assert_eq!(id.default_ip, None);
    }

    #[test]
    fn decodes_code128_fallback() {
        let id = decode("1|DRP-9F2C18|droplet|lora|70B3D5").unwrap();
        assert_eq!(id.id, "DRP-9F2C18");
        assert_eq!(id.model, "droplet");
        assert_eq!(id.network, "lora");
        assert_eq!(id.address.as_deref(), Some("70B3D5"));
    }

    #[test]
    fn decodes_rubix_with_ip_no_address() {
        let id = decode("rubix://add?v=1&id=IO-1&model=io_22&network=rubix&ip=192.168.15.42")
            .unwrap();
        assert_eq!(id.network, "rubix");
        assert_eq!(id.default_ip.as_deref(), Some("192.168.15.42"));
        assert_eq!(id.address, None);
    }

    #[test]
    fn percent_decodes_spaces() {
        assert_eq!(percent_decode("a%20b+c"), "a b c");
    }

    #[test]
    fn rejects_empty_and_garbage() {
        assert!(decode("").is_err());
        assert!(decode("not-a-barcode").is_err());
        assert!(decode("rubix://add?id=X&model=Y").is_err()); // no network
        assert!(decode("1|only|two").is_err());
    }
}
