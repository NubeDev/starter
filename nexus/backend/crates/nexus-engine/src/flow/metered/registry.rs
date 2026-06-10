//! Build a per-flow registry that wraps a flow's source and sink with the metered
//! policy decorators, leaving every other builder untouched.
//!
//! The flow manager knows a run's exact source and sink type names (from the input
//! and output config), so this overrides just those two builders in a fresh
//! `native_registry`: the override delegates to an inner native registry to build
//! the real node, then wraps it in [`MeteredSource`]/[`MeteredSink`]. Processors
//! and any unused built-ins keep their native builders, so a flow's processor
//! chain builds exactly as before. The metered wrappers carry the run's shared
//! [`FlowMetrics`] handle and the parsed failure policy.

use std::sync::Arc;

use serde_json::Value;

use crate::core::Registry;
use crate::flow::metrics::FlowMetrics;
use crate::flow::policy::{sink_policy, source_policy};
use crate::native_registry;

use super::sink::MeteredSink;
use super::source::MeteredSource;

/// Construct a registry for one flow run: native builders everywhere, except the
/// `source_type` and `sink_type` builders are wrapped to count metrics and apply
/// the flow's read/write error policy parsed from `input`/`output`.
pub fn metered_registry(
    source_type: &str,
    sink_type: &str,
    input: &Value,
    output: &Value,
    metrics: FlowMetrics,
) -> Registry {
    let inner = Arc::new(native_registry());
    let mut registry = native_registry();

    let src_policy = source_policy(input);
    let src_metrics = metrics.clone();
    let src_inner = inner.clone();
    let src_type = source_type.to_string();
    registry.register_source(
        source_type,
        Box::new(move |config| {
            let node = src_inner.build_source(&src_type, config)?;
            Ok(Box::new(MeteredSource::new(
                node,
                src_metrics.clone(),
                src_policy.clone(),
            )))
        }),
    );

    let snk_policy = sink_policy(output);
    let snk_metrics = metrics;
    let snk_inner = inner;
    let snk_type = sink_type.to_string();
    registry.register_sink(
        sink_type,
        Box::new(move |config| {
            let node = snk_inner.build_sink(&snk_type, config)?;
            Ok(Box::new(MeteredSink::new(
                node,
                snk_metrics.clone(),
                snk_policy.clone(),
            )))
        }),
    );

    registry
}
