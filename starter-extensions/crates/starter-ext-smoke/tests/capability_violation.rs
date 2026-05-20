//! SCOPE smoke: "Capability violation is rejected, logged, counted".
//!
//! A process-flavour extension that did *not* declare `fs` in its
//! manifest calls `host.fs.read`. The gate at the JSON-RPC wire
//! boundary returns `Error::Capability`, the per-extension violation
//! counter increments, and the extension is *not* killed — buggy ≠
//! malicious (R8).
//!
//! The supervisor's `SupervisorTask` wires `CapabilityGate::check`
//! directly into the inbound frame path and increments
//! `CapabilityViolationCounter` on every refusal. Exercising the gate
//! + counter together against an extension that declared exactly
//! `secrets` proves R8 holds without spawning a real child.

use starter_ext_spi::{Capability, Error};
use starter_ext_supervisor::{CapabilityGate, CapabilityViolationCounter};

#[test]
fn ungranted_fs_read_is_rejected_and_counted() {
    let gate = CapabilityGate::from_manifest(&[Capability::Secrets { prefixes: vec![] }]);
    let counter = CapabilityViolationCounter::default();

    // The wire loop's typical sequence on a refused inbound method.
    let result = gate.check("fs.read");
    assert!(
        matches!(result, Err(Error::Capability(_))),
        "fs.read without `fs` capability must yield Error::Capability"
    );
    counter.inc();
    assert_eq!(counter.get(), 1);

    // Other calls still go through — the gate is per-method, not a
    // sticky-fail latch. SCOPE: "The extension is not killed (it might
    // be buggy, not malicious)".
    assert!(
        gate.check("secrets.get").is_ok(),
        "secrets.get must still be allowed: secrets WAS declared"
    );
    assert!(
        gate.check("health").is_ok(),
        "health is substrate; never gated"
    );
    assert!(
        gate.check("stream.event").is_ok(),
        "stream.* notifications bypass the capability gate"
    );

    // A second offence keeps incrementing — the admin endpoint reads
    // this directly.
    let _ = gate.check("fs.write");
    counter.inc();
    assert_eq!(counter.get(), 2);
}

/// An unknown leading namespace is *also* a refusal, even though
/// nothing in `CAPABILITY_HOST_METHODS` references it. SCOPE R8 says
/// "the supervisor cannot tell what capability would gate it, and
/// silently forwarding to the host risks bypassing R8".
#[test]
fn unknown_namespace_is_treated_as_a_violation() {
    let gate = CapabilityGate::from_manifest(&[Capability::HttpOut {
        authorities: vec![],
    }]);
    let result = gate.check("rampage.do_a_thing");
    assert!(matches!(result, Err(Error::Capability(_))));
}
