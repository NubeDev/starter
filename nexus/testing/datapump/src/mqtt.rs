use std::time::Duration;

use anyhow::{Context, Result};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet};
use tokio::task::JoinHandle;

use crate::config::MqttConfig;
use crate::model::Telemetry;

pub struct MqttPublisher {
    client: AsyncClient,
    eventloop: JoinHandle<()>,
}

impl MqttPublisher {
    pub async fn connect(config: &MqttConfig) -> Result<Self> {
        let mut options = MqttOptions::new(&config.client_id, &config.host, config.port);
        options.set_keep_alive(Duration::from_secs(5));
        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            options.set_credentials(username, password);
        }

        let (client, mut eventloop) = AsyncClient::new(options, 32);
        let eventloop = tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::ConnAck(_))) => {
                        tracing::info!("connected to MQTT broker");
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!(%error, "MQTT event loop ended");
                        break;
                    }
                }
            }
        });

        Ok(Self { client, eventloop })
    }

    pub async fn publish(
        &self,
        path_prefix: &str,
        path_tenant: &str,
        qos: rumqttc::QoS,
        telemetry: &Telemetry,
    ) -> Result<()> {
        let topic = telemetry.path(path_prefix, path_tenant);
        let payload = serde_json::to_vec(telemetry).context("serialize telemetry")?;
        self.client
            .publish(topic, qos, false, payload)
            .await
            .context("publish telemetry")
    }

    pub async fn disconnect(self) -> Result<()> {
        let _ = self.client.disconnect().await;
        self.eventloop.abort();
        Ok(())
    }
}
