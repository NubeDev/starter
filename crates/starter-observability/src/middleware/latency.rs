//! Latency-observation middleware. Records request duration into
//! the [`super::super::metrics::StandardMetrics`] histogram.

/// Build a tower `Layer` that observes request latency.
///
/// Stub — implementation lands with `starter-server` so the service
/// shape is pinned in one place.
pub fn latency_layer() {
    // TODO(ap): see request_id_layer's note.
}
