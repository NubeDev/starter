use anyhow::Result;

use crate::config::{PublishConfig, TransportConfig};
use crate::model::Telemetry;
use crate::mqtt::MqttPublisher;
use crate::zenoh::ZenohPublisher;

pub enum Publisher {
    Mqtt(MqttPublisher),
    Zenoh(ZenohPublisher),
}

impl Publisher {
    pub async fn connect(config: &TransportConfig) -> Result<Self> {
        match config {
            TransportConfig::Mqtt(config) => Ok(Self::Mqtt(MqttPublisher::connect(config).await?)),
            TransportConfig::Zenoh(config) => {
                Ok(Self::Zenoh(ZenohPublisher::connect(config).await?))
            }
        }
    }

    pub async fn publish(
        &self,
        path_prefix: &str,
        path_tenant: &str,
        publish: &PublishConfig,
        telemetry: &Telemetry,
    ) -> Result<()> {
        match self {
            Self::Mqtt(publisher) => {
                publisher
                    .publish(path_prefix, path_tenant, publish.qos, telemetry)
                    .await
            }
            Self::Zenoh(publisher) => publisher.publish(path_prefix, path_tenant, telemetry).await,
        }
    }

    pub async fn disconnect(self) -> Result<()> {
        match self {
            Self::Mqtt(publisher) => publisher.disconnect().await,
            Self::Zenoh(_) => Ok(()),
        }
    }
}
