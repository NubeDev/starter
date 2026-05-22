//! Auto / free port picker, modelled on Vite's dev-server behaviour.
//!
//! - [`pick`] tries a preferred port, then walks forward (`port + 1`,
//!   `port + 2`, …) until it finds one the OS will let us bind, or it
//!   hits the configured attempt cap.
//! - [`pick_strict`] only tries the preferred port and returns
//!   [`PortError::InUse`] if it is taken (Vite's `strictPort: true`).
//! - [`is_free`] is the single-shot probe used by both.
//!
//! The probe binds a `TcpListener` to the requested address and then
//! drops it. This is racy by definition — between the probe and your
//! real bind, another process can grab the port — but it matches what
//! Vite, webpack-dev-server, and friends do, and is good enough for
//! local dev tooling. For production servers, just `bind` directly.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};

use thiserror::Error;

/// Default upper bound on how many consecutive ports [`pick`] will
/// probe before giving up. Matches Vite's behaviour of walking until a
/// free port is found rather than failing fast.
pub const DEFAULT_MAX_ATTEMPTS: u16 = 100;

#[derive(Debug, Error)]
pub enum PortError {
    #[error("port {0} is already in use")]
    InUse(u16),

    #[error("no free port found in range {start}..={end}")]
    Exhausted { start: u16, end: u16 },

    #[error("io error while probing port: {0}")]
    Io(#[from] std::io::Error),
}

/// Options controlling how [`pick_with`] searches for a free port.
#[derive(Debug, Clone, Copy)]
pub struct PickOptions {
    /// Address to bind the probe to. Defaults to `127.0.0.1` — use
    /// `0.0.0.0` if your real server will bind to all interfaces, so
    /// the probe sees the same conflicts the real bind would.
    pub host: IpAddr,

    /// First port to try. The picker walks forward from here.
    pub preferred: u16,

    /// Maximum number of consecutive ports to probe.
    pub max_attempts: u16,

    /// If true, only the preferred port is tried (Vite's `strictPort`).
    pub strict: bool,
}

impl Default for PickOptions {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            preferred: 5173, // Vite's default — nothing magic about it.
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            strict: false,
        }
    }
}

/// Probe a single `(host, port)` by binding and immediately dropping a
/// `TcpListener`. Returns `true` if the bind succeeded.
pub fn is_free(host: IpAddr, port: u16) -> bool {
    TcpListener::bind(SocketAddr::new(host, port)).is_ok()
}

/// Pick a free port on `127.0.0.1`, walking forward from `preferred`.
pub fn pick(preferred: u16) -> Result<u16, PortError> {
    pick_with(PickOptions {
        preferred,
        ..PickOptions::default()
    })
}

/// Try only `preferred` on `127.0.0.1`; error if it is taken.
pub fn pick_strict(preferred: u16) -> Result<u16, PortError> {
    pick_with(PickOptions {
        preferred,
        strict: true,
        ..PickOptions::default()
    })
}

/// Ask the OS for an ephemeral port by binding to port 0. Useful in
/// tests where you don't care about the number, just that it's free
/// for the next instant.
pub fn pick_ephemeral() -> Result<u16, PortError> {
    let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))?;
    Ok(listener.local_addr()?.port())
}

/// Full-control variant. See [`PickOptions`].
pub fn pick_with(opts: PickOptions) -> Result<u16, PortError> {
    if opts.strict {
        return if is_free(opts.host, opts.preferred) {
            Ok(opts.preferred)
        } else {
            Err(PortError::InUse(opts.preferred))
        };
    }

    let start = opts.preferred;
    // Saturating math so a preferred port near u16::MAX doesn't wrap
    // around to 0 (which would mean "ephemeral" and lie about success).
    let end = start.saturating_add(opts.max_attempts.saturating_sub(1));

    for port in start..=end {
        if is_free(opts.host, port) {
            return Ok(port);
        }
    }
    Err(PortError::Exhausted { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_returns_nonzero() {
        let p = pick_ephemeral().unwrap();
        assert!(p > 0);
    }

    #[test]
    fn pick_walks_past_a_taken_port() {
        let host = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let taken = TcpListener::bind(SocketAddr::new(host, 0)).unwrap();
        let busy = taken.local_addr().unwrap().port();

        let found = pick_with(PickOptions {
            host,
            preferred: busy,
            max_attempts: 50,
            strict: false,
        })
        .unwrap();

        assert_ne!(found, busy);
        assert!(found >= busy);
    }

    #[test]
    fn strict_fails_when_taken() {
        let host = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let taken = TcpListener::bind(SocketAddr::new(host, 0)).unwrap();
        let busy = taken.local_addr().unwrap().port();

        let err = pick_with(PickOptions {
            host,
            preferred: busy,
            max_attempts: 1,
            strict: true,
        })
        .unwrap_err();

        assert!(matches!(err, PortError::InUse(p) if p == busy));
    }
}
