//! axum-flavored middleware for starter-server. Data types
//! (`RequestId`, `StandardMetrics`) live in `starter-observability`;
//! the helpers here mount them onto an `axum::Router`.

mod accept_units;
mod latency;
mod request_id;

pub use accept_units::{
    accept_units_layer, with_accept_units, AcceptUnitsLayer, AcceptUnitsService, PrefsResolverFor,
    UnitsCtx, UnitsMode, ACCEPT_UNITS_HEADER,
};
pub use latency::with_latency;
pub use request_id::with_request_id;
