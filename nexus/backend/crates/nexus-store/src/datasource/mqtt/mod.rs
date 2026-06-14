//! The MQTT connector — how nexus probes connectivity to an MQTT broker for a
//! `stream`-surface datasource. Folder-per-connector, a sibling of `postgres/`.
//!
//! MQTT is a *stream* source: it feeds live panels and flows by subscribing to a
//! topic, not an ad-hoc `POST /query`. The only connectivity concern at the store
//! layer is the pre-save connect probe (open a session, confirm it connects,
//! close it). The probe is gated behind the crate's `mqtt` feature (rumqttc, off
//! by default, HOW-TO-CODE §9); with the feature disabled the probe returns a
//! clear "not enabled" error so a build without the connector never fakes success.

mod probe;

pub use probe::{probe, ProbeParams};
