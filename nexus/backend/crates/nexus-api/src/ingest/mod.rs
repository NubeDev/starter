//! Push-ingest domain: enqueue a pushed JSON body onto a running flow's channel.
//!
//! The REST route is a thin shell over [`enqueue`]: it authenticates, then hands
//! the tenant, flow id, and parsed body here. This module owns the two business
//! rules — the flow must belong to the caller's tenant (a cross-tenant push is
//! indistinguishable from a missing flow, so it 404s rather than 403s, matching
//! the rest of the flow surface) and a full channel is backpressure, not an error.

pub mod enqueue;

pub use enqueue::{enqueue, EnqueueError};
