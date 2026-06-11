use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterKind {
    Elec,
    Water,
}

impl MeterKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Elec => "elec",
            Self::Water => "water",
        }
    }

    pub fn secondary_tag(self) -> &'static str {
        match self {
            Self::Elec => "power",
            Self::Water => "reading",
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Self::Elec => "kWh",
            Self::Water => "m3",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Meter {
    pub site_id: String,
    pub host_uuid: String,
    pub meter_id: String,
    pub point_uuid: String,
    pub kind: MeterKind,
    pub base: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Telemetry {
    pub tenant_id: String,
    pub site_id: String,
    pub host_uuid: String,
    pub point_uuid: String,
    pub meter_id: String,
    pub kind: String,
    pub secondary_tag: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
}

impl Telemetry {
    pub fn path(&self, prefix: &str, path_tenant: &str) -> String {
        format!(
            "{}/{}/{}/{}/{}",
            prefix.trim_matches('/'),
            path_tenant,
            self.site_id,
            self.kind,
            self.meter_id
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn path_uses_stable_segments() {
        let telemetry = Telemetry {
            tenant_id: "*".into(),
            site_id: "site-001".into(),
            host_uuid: "host-001".into(),
            point_uuid: "point-001".into(),
            meter_id: "meter-001".into(),
            kind: "elec".into(),
            secondary_tag: "power".into(),
            value: 42.0,
            unit: "kWh".into(),
            timestamp: Utc.timestamp_opt(0, 0).single().expect("valid epoch"),
        };

        assert_eq!(
            telemetry.path("/rubix/testing/", "all"),
            "rubix/testing/all/site-001/elec/meter-001"
        );
    }
}
