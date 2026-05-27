//! Wire types for the `dashboard.read` and `dashboard.write` host
//! methods.
//!
//! Per
//! [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
//! row 5: an extension reaches the host's SDUI dashboard store
//! through `ctx.dashboard().read(page_id)` /
//! `.write(page_id, body)`. For process-flavour and WASM-flavour
//! extensions, the SDK handle marshals those calls as JSON-RPC
//! requests on the substrate's host-method channel; the supervisor
//! routes them into the host's [`super::Capability::DashboardRead`]
//! and [`super::Capability::DashboardWrite`] backends after the
//! capability gate fires.
//!
//! Builtin-flavour extensions never hit the wire; they call the
//! host's backend directly through `Arc<dyn DashboardBackend>`.
//!
//! Naming follows the existing
//! [`super::warehouse::WarehouseReadRequest`] precedent: one
//! struct per host method, both halves of the request live here
//! (`Request` + `Response`) so adapters can deserialise without
//! pulling the SDK in.

use serde::{Deserialize, Serialize};

/// Wire payload an extension sends on `dashboard.read`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardReadRequest {
    /// Stable SDUI page id. The host clamps the read to the
    /// caller's tenant id from `_meta.caller.tenant_id`;
    /// extensions cannot read another tenant's page.
    pub page_id: String,
}

/// Wire response for `dashboard.read`.
///
/// The body is opaque JSON (the SDUI layer owns the schema). The
/// host returns `Error::ExtensionInternal` when the page does not
/// exist for the caller's tenant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardReadResponse {
    /// Raw body the host stored. The SDK handle hands this back
    /// verbatim from `read(page_id) -> serde_json::Value`.
    pub body: serde_json::Value,
}

/// Wire payload an extension sends on `dashboard.write`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardWriteRequest {
    /// Stable SDUI page id. Same tenancy clamp as
    /// [`DashboardReadRequest`].
    pub page_id: String,
    /// New page body. Replaces any prior body in the same
    /// transaction the host's `DashboardStore::insert_revision`
    /// runs; the prior row is superseded, not deleted.
    pub body: serde_json::Value,
}

/// Wire response for `dashboard.write`. Empty struct rather than
/// `()` so future fields (revision id, supersede info) can land
/// additively.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardWriteResponse {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_read_request_round_trip() {
        let req = DashboardReadRequest {
            page_id: "dashboard.disk-overview".into(),
        };
        let j = serde_json::to_value(&req).unwrap();
        assert_eq!(j["page_id"], "dashboard.disk-overview");
        let back: DashboardReadRequest = serde_json::from_value(j).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn dashboard_write_request_round_trip() {
        let req = DashboardWriteRequest {
            page_id: "dashboard.disk-overview".into(),
            body: serde_json::json!({"v": 1}),
        };
        let j = serde_json::to_value(&req).unwrap();
        let back: DashboardWriteRequest = serde_json::from_value(j).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn dashboard_write_response_round_trips_empty() {
        let res = DashboardWriteResponse::default();
        let j = serde_json::to_value(&res).unwrap();
        let back: DashboardWriteResponse = serde_json::from_value(j).unwrap();
        assert_eq!(back, res);
    }
}
