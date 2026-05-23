//! `rubix-agent` — Phase-0 skeleton binary. Prints its version and exits 0.
//! Real wiring (graph + engine + kinds registry) lands in Phase 1+.

fn main() {
    println!("rubix-agent v{}", env!("CARGO_PKG_VERSION"));
}
