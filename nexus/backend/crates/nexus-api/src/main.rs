//! Nexus control-plane server entrypoint.
//!
//! Composes the auth/authz routers from the starter crates with nexus's product
//! routers over the engine and store, then serves. Fleshed out as the route
//! work-units land; M0 wires the query path.

fn main() {
    eprintln!("nexus-api: server wiring lands with the M0 route work-units");
}
