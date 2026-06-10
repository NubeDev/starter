use clap::Parser;

use crate::config::TransportKind;

/// Publish synthetic Rubix-style meter telemetry to MQTT or Zenoh.
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Args {
    /// Output transport: mqtt, zenoh, or zenho.
    #[arg(long, env = "DATAPUMP_TRANSPORT", default_value = "mqtt")]
    pub transport: TransportKind,

    /// MQTT broker host.
    #[arg(long, env = "MQTT_HOST", default_value = "127.0.0.1")]
    pub host: String,

    /// MQTT broker port.
    #[arg(long, env = "MQTT_PORT", default_value_t = 1883)]
    pub port: u16,

    /// MQTT client id.
    #[arg(long, env = "MQTT_CLIENT_ID", default_value = "nexus-mqtt-data-gen")]
    pub client_id: String,

    /// Optional MQTT username.
    #[arg(long, env = "MQTT_USERNAME")]
    pub username: Option<String>,

    /// Optional MQTT password.
    #[arg(long, env = "MQTT_PASSWORD")]
    pub password: Option<String>,

    /// Zenoh endpoint to connect to.
    #[arg(long, env = "ZENOH_ENDPOINT", default_value = "tcp/127.0.0.1:7447")]
    pub zenoh_endpoint: String,

    /// Prefix before tenant/site/kind/meter path segments.
    #[arg(
        long = "path-prefix",
        alias = "topic-prefix",
        env = "DATAPUMP_PATH_PREFIX",
        default_value = "rubix/testing"
    )]
    pub path_prefix: String,

    /// Tenant id written into each JSON payload.
    #[arg(long, env = "RUBIX_TENANT_ID", default_value = "*")]
    pub tenant_id: String,

    /// Tenant path segment used in MQTT topics and Zenoh key expressions.
    #[arg(long, env = "RUBIX_TOPIC_TENANT", default_value = "all")]
    pub path_tenant: String,

    /// Number of synthetic sites.
    #[arg(long, env = "RUBIX_SITE_COUNT", default_value_t = 3)]
    pub sites: usize,

    /// Number of electric and water meters to emit per site.
    #[arg(long, env = "RUBIX_METERS_PER_KIND", default_value_t = 4)]
    pub meters_per_kind: usize,

    /// Publish interval in milliseconds.
    #[arg(long, env = "RUBIX_INTERVAL_MS", default_value_t = 1_000)]
    pub interval_ms: u64,

    /// Optional total publish count. Omit to publish until Ctrl-C.
    #[arg(long, env = "RUBIX_COUNT")]
    pub count: Option<u64>,

    /// Deterministic RNG seed.
    #[arg(long, env = "RUBIX_SEED", default_value_t = 42)]
    pub seed: u64,

    /// MQTT QoS: 0, 1, or 2.
    #[arg(long, env = "MQTT_QOS", default_value_t = 0)]
    pub qos: u8,
}
