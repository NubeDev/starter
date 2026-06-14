use std::str::FromStr;
use std::time::Duration;

use anyhow::{bail, Result};
use rumqttc::QoS;

use crate::cli::Args;

#[derive(Debug, Clone)]
pub struct Config {
    pub transport: TransportConfig,
    pub shape: ShapeConfig,
    pub publish: PublishConfig,
}

#[derive(Debug, Clone)]
pub enum TransportConfig {
    Mqtt(MqttConfig),
    Zenoh(ZenohConfig),
}

impl TransportConfig {
    pub fn kind(&self) -> TransportKind {
        match self {
            Self::Mqtt(_) => TransportKind::Mqtt,
            Self::Zenoh(_) => TransportKind::Zenoh,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ZenohConfig {
    pub endpoint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Mqtt,
    Zenoh,
}

impl FromStr for TransportKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "mqtt" => Ok(Self::Mqtt),
            "zenoh" | "zenho" => Ok(Self::Zenoh),
            _ => Err("transport must be mqtt or zenoh".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShapeConfig {
    pub path_prefix: String,
    pub tenant_id: String,
    pub path_tenant: String,
    pub sites: usize,
    pub meters_per_kind: usize,
    pub seed: u64,
}

#[derive(Debug, Clone)]
pub struct PublishConfig {
    pub interval: Duration,
    pub count: Option<u64>,
    pub qos: QoS,
}

impl TryFrom<Args> for Config {
    type Error = anyhow::Error;

    fn try_from(args: Args) -> Result<Self> {
        if args.sites == 0 {
            bail!("sites must be greater than zero");
        }
        if args.meters_per_kind == 0 {
            bail!("meters-per-kind must be greater than zero");
        }
        if args.interval_ms == 0 {
            bail!("interval-ms must be greater than zero");
        }
        if args.password.is_some() && args.username.is_none() {
            bail!("password requires username");
        }
        if args.zenoh_endpoint.trim().is_empty() {
            bail!("zenoh-endpoint must not be empty");
        }

        Ok(Self {
            transport: match args.transport {
                TransportKind::Mqtt => TransportConfig::Mqtt(MqttConfig {
                    host: args.host,
                    port: args.port,
                    client_id: args.client_id,
                    username: args.username,
                    password: args.password,
                }),
                TransportKind::Zenoh => TransportConfig::Zenoh(ZenohConfig {
                    endpoint: args.zenoh_endpoint,
                }),
            },
            shape: ShapeConfig {
                path_prefix: trim_path(&args.path_prefix),
                tenant_id: args.tenant_id,
                path_tenant: sanitize_path_segment(&args.path_tenant),
                sites: args.sites,
                meters_per_kind: args.meters_per_kind,
                seed: args.seed,
            },
            publish: PublishConfig {
                interval: Duration::from_millis(args.interval_ms),
                count: args.count,
                qos: qos(args.qos)?,
            },
        })
    }
}

impl std::fmt::Display for TransportKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mqtt => f.write_str("mqtt"),
            Self::Zenoh => f.write_str("zenoh"),
        }
    }
}

fn qos(value: u8) -> Result<QoS> {
    match value {
        0 => Ok(QoS::AtMostOnce),
        1 => Ok(QoS::AtLeastOnce),
        2 => Ok(QoS::ExactlyOnce),
        _ => bail!("qos must be 0, 1, or 2"),
    }
}

fn trim_path(value: &str) -> String {
    value.trim_matches('/').to_string()
}

fn sanitize_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '/' | '+' | '#' | '*' | ' ' => '_',
            _ => ch,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_counts() {
        let args = Args {
            host: "127.0.0.1".into(),
            port: 1883,
            client_id: "test".into(),
            username: None,
            password: None,
            zenoh_endpoint: "tcp/127.0.0.1:7447".into(),
            transport: TransportKind::Mqtt,
            path_prefix: "rubix/testing".into(),
            tenant_id: "*".into(),
            path_tenant: "all".into(),
            sites: 0,
            meters_per_kind: 1,
            interval_ms: 1_000,
            count: None,
            seed: 42,
            qos: 0,
        };

        assert!(Config::try_from(args).is_err());
    }

    #[test]
    fn sanitizes_path_tenant() {
        assert_eq!(sanitize_path_segment("*"), "_");
        assert_eq!(sanitize_path_segment("a/b + c"), "a_b___c");
    }

    #[test]
    fn accepts_zenho_alias() {
        assert_eq!(TransportKind::from_str("zenho"), Ok(TransportKind::Zenoh));
    }
}
