use chrono::Utc;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::config::ShapeConfig;
use crate::model::{Meter, MeterKind, Telemetry};

pub struct Generator {
    tenant_id: String,
    meters: Vec<Meter>,
    rng: StdRng,
    next_meter: usize,
}

impl Generator {
    pub fn new(config: &ShapeConfig) -> Self {
        Self {
            tenant_id: config.tenant_id.clone(),
            meters: build_meters(config),
            rng: StdRng::seed_from_u64(config.seed),
            next_meter: 0,
        }
    }

    pub fn next(&mut self) -> Telemetry {
        let idx = self.next_meter;
        self.next_meter = (self.next_meter + 1) % self.meters.len();

        let meter = &self.meters[idx];
        let value = reading(meter, &mut self.rng);
        Telemetry {
            tenant_id: self.tenant_id.clone(),
            site_id: meter.site_id.clone(),
            host_uuid: meter.host_uuid.clone(),
            point_uuid: meter.point_uuid.clone(),
            meter_id: meter.meter_id.clone(),
            kind: meter.kind.as_str().to_string(),
            secondary_tag: meter.kind.secondary_tag().to_string(),
            value,
            unit: meter.kind.unit().to_string(),
            timestamp: Utc::now(),
        }
    }

    pub fn meter_count(&self) -> usize {
        self.meters.len()
    }
}

fn build_meters(config: &ShapeConfig) -> Vec<Meter> {
    let mut meters = Vec::with_capacity(config.sites * config.meters_per_kind * 2);
    for site in 1..=config.sites {
        let site_id = format!("site-{site:03}");
        let host_uuid = format!("host-{site:03}");
        for ordinal in 1..=config.meters_per_kind {
            meters.push(meter(&site_id, &host_uuid, MeterKind::Elec, ordinal));
            meters.push(meter(&site_id, &host_uuid, MeterKind::Water, ordinal));
        }
    }
    meters
}

fn meter(site_id: &str, host_uuid: &str, kind: MeterKind, ordinal: usize) -> Meter {
    let meter_id = format!("{}-{}-{ordinal:03}", site_id, kind.as_str());
    let base = match kind {
        MeterKind::Elec => 25.0 + ordinal as f64 * 3.5,
        MeterKind::Water => 1.0 + ordinal as f64 * 0.35,
    };

    Meter {
        site_id: site_id.to_string(),
        host_uuid: host_uuid.to_string(),
        point_uuid: format!("{meter_id}-point"),
        meter_id,
        kind,
        base,
    }
}

fn reading(meter: &Meter, rng: &mut StdRng) -> f64 {
    let jitter = match meter.kind {
        MeterKind::Elec => rng.gen_range(-3.0..12.0),
        MeterKind::Water => rng.gen_range(-0.15..1.25),
    };
    round_3((meter.base + jitter).max(0.0))
}

fn round_3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rumqttc::QoS;

    use crate::config::{Config, MqttConfig, PublishConfig, TransportConfig};

    use super::*;

    fn config() -> Config {
        Config {
            transport: TransportConfig::Mqtt(MqttConfig {
                host: "127.0.0.1".into(),
                port: 1883,
                client_id: "test".into(),
                username: None,
                password: None,
            }),
            shape: ShapeConfig {
                path_prefix: "rubix/testing".into(),
                tenant_id: "*".into(),
                path_tenant: "all".into(),
                sites: 2,
                meters_per_kind: 3,
                seed: 1,
            },
            publish: PublishConfig {
                interval: Duration::from_millis(1),
                count: Some(1),
                qos: QoS::AtMostOnce,
            },
        }
    }

    #[test]
    fn builds_electric_and_water_meters_per_site() {
        let generator = Generator::new(&config().shape);

        assert_eq!(generator.meter_count(), 12);
    }

    #[test]
    fn emits_warehouse_compatible_payload_fields() {
        let mut generator = Generator::new(&config().shape);
        let row = generator.next();

        assert_eq!(row.tenant_id, "*");
        assert_eq!(row.site_id, "site-001");
        assert_eq!(row.host_uuid, "host-001");
        assert_eq!(row.kind, "elec");
        assert_eq!(row.secondary_tag, "power");
        assert!(row.value >= 0.0);
    }
}
