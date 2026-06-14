//! Peer-supervisor registry for `extension.call` (WS-18 Wave B).
//!
//! `extension.call` must reach the **callee** extension's running child to run
//! a provided tool/node and return its output. The per-extension
//! [`SupervisorHandle`]s are spawned by [`super::boot`] *after* `AppState` (and
//! thus the host-method handler that closes over it) is built — the same
//! chicken/egg the boot module already navigates for `extension_kinds`.
//!
//! This module resolves it with a write-once cell: `AppState` carries an
//! `Arc<PeerSupervisors>` from construction; `boot` fills it in once, after the
//! supervisors are spawned. Because every `AppState` clone shares the one `Arc`,
//! the host-method handler sees the populated registry the moment boot sets it —
//! no restructuring of the boot order, no `Mutex` on the hot path.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use starter_ext_spi::extension::{ExtensionCallRequest, ExtensionCallResponse};
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::{Capability, Error as ExtError, ExtensionId, Result as ExtResult};
use starter_ext_supervisor::SupervisorHandle;

use crate::state::AppState;

/// Wall-clock bound on a single peer call. A callee that does not answer within
/// this window fails the call with a transport timeout rather than hanging the
/// caller. It also bounds runaway call chains: each hop consumes its own window,
/// so a cycle cannot livelock the runtime indefinitely.
const PEER_CALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Write-once registry of `extension_id -> SupervisorHandle` for the enabled
/// process-flavour extensions, shared between `AppState` and `boot` via an
/// `Arc`. Empty until [`set`](Self::set) is called once at the end of boot.
#[derive(Default)]
pub struct PeerSupervisors {
    handles: OnceLock<HashMap<String, SupervisorHandle>>,
}

impl PeerSupervisors {
    /// Build the empty cell placed on `AppState` before boot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Populate the registry exactly once, at the end of boot. A second call is
    /// ignored (returns the supplied map back to the caller) — boot runs once.
    pub fn set(
        &self,
        handles: HashMap<String, SupervisorHandle>,
    ) -> Result<(), HashMap<String, SupervisorHandle>> {
        self.handles.set(handles)
    }

    /// Look up the running child for `extension_id`. `None` if boot has not
    /// populated the registry yet, the extension is not a running process, or
    /// it is disabled/absent.
    pub fn get(&self, extension_id: &str) -> Option<&SupervisorHandle> {
        self.handles.get()?.get(extension_id)
    }
}

/// `extension.call` — synchronously invoke a peer extension's provided
/// tool/node under the **caller's** identity (WS-18 Wave B).
///
/// The dispatch is gated three ways, all enforced here before any wire traffic:
/// 1. **Caller grant.** `"<callee>:<provided_id>"` must be in the caller's
///    `Capability::Extension { targets }`.
/// 2. **Caller declaration.** The caller must list the callee + provided id in
///    `requires_extensions[]` (the operator-visible peer edge).
/// 3. **Callee opt-in.** The callee must list the provided id in
///    `contributes.provides[]` — a tool/node it published as peer-callable.
///
/// Only then is the input forwarded to the callee's child as a `tools/<id>`
/// dispatch, **carrying the caller's tenant/teams** (never the callee's), so the
/// callee can reach no data the caller could not. A tenant-less caller is a hard
/// deny; a hung callee trips [`PEER_CALL_TIMEOUT`].
pub async fn call(
    state: &AppState,
    extension: &ExtensionId,
    params: serde_json::Value,
    caller: Option<&CallerIdentity>,
) -> ExtResult<serde_json::Value> {
    let req: ExtensionCallRequest = serde_json::from_value(params)
        .map_err(|e| ExtError::extension_internal(format!("extension.call params: {e}")))?;

    // Tenant-scoped: a caller with no tenant cannot make a peer call (the callee
    // would run unscoped). Hard deny, like every tenant-bound host method.
    let caller = caller.ok_or_else(|| {
        ExtError::extension_internal("extension.call requires a caller identity (none supplied)")
    })?;
    if caller.tenant_id.is_none() {
        return Err(ExtError::extension_internal(
            "extension.call requires a tenant-scoped caller (tenant_id is None)",
        ));
    }

    // Resolve both manifests and run the three gates (caller grant + caller
    // declaration + callee opt-in) — a pure decision so it is unit-testable
    // without standing up an `AppState`.
    let caller_manifest = state
        .extensions
        .get_by_id_str(extension.as_str())
        .and_then(|r| r.manifest.as_ref())
        .ok_or_else(|| {
            ExtError::extension_internal(format!(
                "extension.call: calling extension {:?} has no loaded manifest",
                extension.as_str()
            ))
        })?;
    let callee_manifest = state
        .extensions
        .get_by_id_str(&req.extension_id)
        .and_then(|r| r.manifest.as_ref())
        .ok_or_else(|| {
            ExtError::extension_internal(format!(
                "extension.call: callee extension {:?} is not installed",
                req.extension_id
            ))
        })?;
    check_gates(
        caller_manifest,
        callee_manifest,
        extension.as_str(),
        &req.extension_id,
        &req.provided_id,
    )?;

    // Resolve the callee's running child.
    let handle = state
        .peer_supervisors
        .get(&req.extension_id)
        .ok_or_else(|| {
            ExtError::extension_internal(format!(
                "extension.call: callee {:?} is not a running process extension",
                req.extension_id
            ))
        })?;

    // Forward as a tool dispatch, stamping the CALLER's identity so the callee
    // runs under the caller's tenant/teams — never its own. `provides[]` ids are
    // contributed tool ids or node kinds; both route through the child's
    // `dispatch_tool` via the `tools/<id>` wire method.
    let method = format!("tools/{}", req.provided_id);
    let output = handle
        .call_as(&method, req.input, caller.clone(), PEER_CALL_TIMEOUT)
        .await?;

    serde_json::to_value(ExtensionCallResponse { output })
        .map_err(|e| ExtError::extension_internal(format!("extension.call response: {e}")))
}

/// The pure triple-gate for `extension.call` (WS-18 Wave B). All three must
/// hold; any miss is a capability denial. Separated from [`call`] so the policy
/// is unit-testable without an `AppState` or a running child.
fn check_gates(
    caller_manifest: &starter_ext_spi::Manifest,
    callee_manifest: &starter_ext_spi::Manifest,
    caller_id: &str,
    callee_id: &str,
    provided_id: &str,
) -> ExtResult<()> {
    let target = format!("{callee_id}:{provided_id}");

    // (1) Caller grant: "<callee>:<provided_id>" must be allowlisted.
    let granted = caller_manifest.capabilities.iter().any(|c| match c {
        Capability::Extension { targets } => targets.iter().any(|t| t == &target),
        _ => false,
    });
    if !granted {
        return Err(ExtError::capability(format!(
            "extension.call: target {target:?} is not in extension {caller_id:?}'s `extension` grant"
        )));
    }

    // (2) Caller declaration: the peer edge must be declared in
    //     requires_extensions[] (operator-visible, no hidden edges).
    let declared = caller_manifest
        .requires_extensions
        .iter()
        .any(|r| r.id.as_str() == callee_id && r.provides.iter().any(|p| p == provided_id));
    if !declared {
        return Err(ExtError::capability(format!(
            "extension.call: {target:?} is granted but not declared in extension {caller_id:?}'s \
             requires_extensions[] (WS-18: peer edges must be declared)"
        )));
    }

    // (3) Callee opt-in: the provided id must be in the callee's
    //     contributes.provides[] (a tool/node it published as peer-callable).
    let provided = callee_manifest
        .contributes
        .provides
        .iter()
        .any(|p| p.id == provided_id);
    if !provided {
        return Err(ExtError::capability(format!(
            "extension.call: extension {callee_id:?} does not `provide` {provided_id:?}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_spi::Manifest;

    fn caller(yaml_caps_and_reqs: &str) -> Manifest {
        let yaml = format!(
            "v: 1\nid: com.acme.sites\nversion: 0.1.0\ndisplay_name: S\n\
             runtime: {{ kind: process, bin: ./b }}\n{yaml_caps_and_reqs}"
        );
        serde_yaml::from_str(&yaml).expect("caller manifest parses")
    }

    fn callee(provides: bool) -> Manifest {
        let provides_block = if provides {
            "contributes:\n  provides:\n    - id: com.acme.geocode.lookup\n"
        } else {
            "contributes: {}\n"
        };
        let yaml = format!(
            "v: 1\nid: com.acme.geocode\nversion: 0.1.0\ndisplay_name: G\n\
             runtime: {{ kind: process, bin: ./b }}\n{provides_block}"
        );
        serde_yaml::from_str(&yaml).expect("callee manifest parses")
    }

    const FULL_CALLER: &str = "\
capabilities:\n  - kind: extension\n    targets: [com.acme.geocode:com.acme.geocode.lookup]\n\
requires_extensions:\n  - id: com.acme.geocode\n    provides: [com.acme.geocode.lookup]\n";

    #[test]
    fn all_three_gates_pass() {
        let c = caller(FULL_CALLER);
        let e = callee(true);
        check_gates(
            &c,
            &e,
            "com.acme.sites",
            "com.acme.geocode",
            "com.acme.geocode.lookup",
        )
        .expect("all gates pass");
    }

    #[test]
    fn missing_grant_is_denied() {
        // Declared + provided, but no `extension` capability target.
        let c = caller(
            "requires_extensions:\n  - id: com.acme.geocode\n    provides: [com.acme.geocode.lookup]\n",
        );
        let e = callee(true);
        let err = check_gates(
            &c,
            &e,
            "com.acme.sites",
            "com.acme.geocode",
            "com.acme.geocode.lookup",
        )
        .unwrap_err();
        assert!(matches!(err, ExtError::Capability(_)));
    }

    #[test]
    fn missing_declaration_is_denied() {
        // Granted + provided, but not declared in requires_extensions[].
        let c = caller(
            "capabilities:\n  - kind: extension\n    targets: [com.acme.geocode:com.acme.geocode.lookup]\n",
        );
        let e = callee(true);
        let err = check_gates(
            &c,
            &e,
            "com.acme.sites",
            "com.acme.geocode",
            "com.acme.geocode.lookup",
        )
        .unwrap_err();
        assert!(matches!(err, ExtError::Capability(_)));
    }

    #[test]
    fn callee_not_providing_is_denied() {
        // Granted + declared, but the callee does not `provide` it.
        let c = caller(FULL_CALLER);
        let e = callee(false);
        let err = check_gates(
            &c,
            &e,
            "com.acme.sites",
            "com.acme.geocode",
            "com.acme.geocode.lookup",
        )
        .unwrap_err();
        assert!(matches!(err, ExtError::Capability(_)));
    }
}
