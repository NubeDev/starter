//! Probe reachability of a Zenoh fabric from *raw* config, before it is saved.
//!
//! Mirrors the MQTT probe's contract: build a one-shot session from the supplied
//! config, force a real round-trip (here: open the session against the configured
//! endpoints), and drop it — so the "Test connection" form can report success
//! before the datasource is persisted. A Zenoh subscriber holds no secret, so
//! there is nothing to seal or audit; only the endpoint reachability is checked.
//!
//! The probe is compiled only when the crate's `zenoh` feature is enabled. With
//! the feature off, [`probe`] returns a clear "not enabled" `Error::Invalid`
//! rather than a fake success — a deployment that did not build the connector
//! cannot silently report a fabric as reachable.

use starter_spi::Error;

/// Raw Zenoh connection parameters for a pre-save probe. Mirrors the config the
/// `zenoh` datasource-kind declares; the route maps its config onto this. The key
/// expression is not needed to test reachability — opening the session is the
/// proof — so the probe takes only the transport parameters.
pub struct ProbeParams<'a> {
    /// Endpoints to connect to, e.g. `["tcp/127.0.0.1:7447"]`. May be empty in
    /// `peer` mode for an in-process mesh.
    pub endpoints: &'a [String],
    /// Session mode: `"client"` (connect to a router) or `"peer"` (mesh directly).
    pub mode: &'a str,
}

/// Open a short-lived Zenoh session against the described fabric and confirm it
/// comes up, then drop it. Returns `Ok(())` once the session opens, and an error
/// carrying the reason otherwise — the route sanitizes that reason before it
/// reaches the client. The session is closed before returning so a probe never
/// holds a transport open against the fabric.
#[cfg(feature = "zenoh")]
pub async fn probe(params: ProbeParams<'_>) -> Result<(), Error> {
    use std::time::Duration;

    // Build the same config shape the engine source uses: set the mode and, when
    // given, the connect endpoints. An invalid endpoint string is a probe failure
    // (Invalid), never a false Ok.
    let mut cfg = zenoh::Config::default();
    cfg.insert_json5("mode", &format!("\"{}\"", params.mode))
        .map_err(|e| Error::Invalid {
            message: format!("zenoh mode rejected: {e}"),
        })?;
    if !params.endpoints.is_empty() {
        let json = serde_json::to_string(params.endpoints).map_err(|e| Error::Invalid {
            message: format!("zenoh endpoints not serializable: {e}"),
        })?;
        cfg.insert_json5("connect/endpoints", &json)
            .map_err(|e| Error::Invalid {
                message: format!("zenoh endpoints rejected: {e}"),
            })?;
    }

    // Bound the open so an unreachable router can't hang the form indefinitely.
    let result = tokio::time::timeout(Duration::from_secs(10), zenoh::open(cfg)).await;

    match result {
        Ok(Ok(session)) => {
            let _ = session.close().await;
            Ok(())
        }
        Ok(Err(e)) => Err(Error::Internal {
            source: Box::new(std::io::Error::other(e.to_string())),
        }),
        Err(_) => Err(Error::Invalid {
            message: "connection probe timed out".into(),
        }),
    }
}

/// Feature-off stand-in: the `zenoh` connector was not compiled into this build,
/// so there is no session to probe with. Returns a clear error rather than a fake
/// success — a deployment that did not opt into the connector must not report a
/// fabric as reachable.
#[cfg(not(feature = "zenoh"))]
pub async fn probe(_params: ProbeParams<'_>) -> Result<(), Error> {
    Err(Error::Invalid {
        message: "the Zenoh connector is not enabled in this build (rebuild with the `zenoh` \
                  feature)"
            .into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With the feature off, the probe must refuse rather than pretend a fabric
    /// connected. (Behind `zenoh` this case is replaced by a real-session test.)
    #[cfg(not(feature = "zenoh"))]
    #[tokio::test]
    async fn probe_without_feature_reports_not_enabled() {
        let err = probe(ProbeParams {
            endpoints: &["tcp/127.0.0.1:7447".to_string()],
            mode: "client",
        })
        .await
        .expect_err("the disabled connector cannot connect");
        assert!(matches!(err, Error::Invalid { .. }));
        assert!(err.to_string().contains("not enabled"));
    }

    /// With the feature on, a peer-mode session with no endpoints opens an
    /// in-process mesh and succeeds — exercising the real open/close path without
    /// an external router.
    #[cfg(feature = "zenoh")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn probe_peer_mode_opens_and_closes() {
        probe(ProbeParams {
            endpoints: &[],
            mode: "peer",
        })
        .await
        .expect("an in-process peer session opens");
    }
}
