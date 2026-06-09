//! Probe connectivity to an MQTT broker from *raw* config, before it is saved.
//!
//! Mirrors the Postgres probe's contract: build a one-shot session from the
//! supplied config, force a real round-trip (here: drive the event loop until the
//! broker's `ConnAck`), and drop it — so the "Test connection" form can report
//! success before the datasource is persisted. The plaintext credentials are used
//! transiently and never logged, cached, or returned.
//!
//! The probe is compiled only when the crate's `mqtt` feature is enabled. With
//! the feature off, [`probe`] returns a clear "not enabled" `Error::Invalid`
//! rather than a fake success — a deployment that did not build the connector
//! cannot silently report a broker as reachable.

use starter_spi::Error;

/// Raw MQTT connection parameters for a pre-save probe. Mirrors the config fields
/// the `mqtt` datasource-kind declares; the route maps its config onto this.
pub struct ProbeParams<'a> {
    /// Broker host.
    pub host: &'a str,
    /// Broker port (1883 plain / 8883 TLS, per the config default).
    pub port: u16,
    /// Client id to connect as; a stable id avoids the broker churning sessions.
    pub client_id: &'a str,
    /// Optional username for authenticated brokers.
    pub user: Option<&'a str>,
    /// Optional password, paired with `user`.
    pub password: Option<&'a str>,
}

/// Open a short-lived MQTT session to the described broker and confirm it
/// connects, then drop it. Returns `Ok(())` once the broker acknowledges the
/// connection, and an error carrying the client's reason otherwise — the route
/// sanitizes that reason before it reaches the client.
#[cfg(feature = "mqtt")]
pub async fn probe(params: ProbeParams<'_>) -> Result<(), Error> {
    use std::time::Duration;

    use rumqttc::{AsyncClient, Event, MqttOptions, Packet};

    let mut opts = MqttOptions::new(params.client_id, params.host, params.port);
    opts.set_keep_alive(Duration::from_secs(5));
    if let (Some(user), Some(pass)) = (params.user, params.password) {
        opts.set_credentials(user, pass);
    }

    // A small capacity is plenty for a probe; we only need to see the ConnAck.
    let (client, mut eventloop) = AsyncClient::new(opts, 8);

    // Bound the probe so a black-holed broker can't hang the form indefinitely.
    // The first ConnAck proves the address + credentials; any earlier error (DNS,
    // refused, auth) surfaces from `poll` as the connection failure reason.
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => return Ok(()),
                Ok(_) => continue,
                Err(e) => return Err(Error::Internal { source: Box::new(e) }),
            }
        }
    })
    .await;

    // Drop the client so the background session tears down; ignore the disconnect
    // result — the probe outcome is decided by the ConnAck above.
    let _ = client.disconnect().await;

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(Error::Invalid {
            message: "connection probe timed out".into(),
        }),
    }
}

/// Feature-off stand-in: the `mqtt` connector was not compiled into this build,
/// so there is no client to probe with. Returns a clear error rather than a fake
/// success — a deployment that did not opt into the connector must not report a
/// broker as reachable.
#[cfg(not(feature = "mqtt"))]
pub async fn probe(_params: ProbeParams<'_>) -> Result<(), Error> {
    Err(Error::Invalid {
        message: "the MQTT connector is not enabled in this build (rebuild with the `mqtt` feature)"
            .into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With the feature off, the probe must refuse rather than pretend a broker
    /// connected. (Behind `mqtt` this case is replaced by the closed-port test.)
    #[cfg(not(feature = "mqtt"))]
    #[tokio::test]
    async fn probe_without_feature_reports_not_enabled() {
        let err = probe(ProbeParams {
            host: "127.0.0.1",
            port: 1883,
            client_id: "nexus-probe",
            user: None,
            password: None,
        })
        .await
        .expect_err("the disabled connector cannot connect");
        assert!(matches!(err, Error::Invalid { .. }));
        assert!(err.to_string().contains("not enabled"));
    }

    /// With the feature on, a probe against a closed local port fails fast (the
    /// broker is unreachable) — exercises the real client error path without a
    /// broker. The keep-alive + 10s timeout bound the wait.
    #[cfg(feature = "mqtt")]
    #[tokio::test]
    async fn probe_to_closed_port_fails() {
        let err = probe(ProbeParams {
            // Port 1 is never an MQTT broker; the connect is refused.
            host: "127.0.0.1",
            port: 1,
            client_id: "nexus-probe",
            user: None,
            password: None,
        })
        .await
        .expect_err("a closed port is not a reachable broker");
        // Either an immediate connection error or the bounded timeout — both are
        // a failed probe, never a false Ok.
        assert!(matches!(err, Error::Internal { .. } | Error::Invalid { .. }));
    }
}
