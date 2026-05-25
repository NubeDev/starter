//! dataflow goal — synthesis tool(s).
//!
//! See `rubix/docs/sessions/data-flow/01-producer.md` for the
//! framework: synthesis is a tool, delivery is a flow. Replay /
//! fixture / load-test sources should land as siblings of `synth`
//! here when they arrive.

pub mod mess;
pub mod meters;
pub mod synth;

#[cfg(test)]
mod tests;
