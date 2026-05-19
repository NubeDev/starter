//! [`build_grpc_methods`] — walks the registry, returns one
//! [`GrpcMethod`] descriptor per `contributes.grpc` entry across every
//! `Validated` extension.
//!
//! The descriptors are what [`crate::service::ExtensionGrpcService`]
//! resolves against at request time. The adapter performs every load-
//! time check (collision, schema I/O, namespace ownership has already
//! been checked by `starter-ext-host::validate`) up front, so the
//! request path never touches the filesystem.

use std::collections::HashMap;
use std::path::PathBuf;

use starter_ext_host::ExtensionRegistry;
use starter_ext_spi::{ContributeGrpc, ExtensionId, LifecycleState};

/// One ready-to-serve gRPC method descriptor.
///
/// Carries the manifest-declared `(service, method)` pair (the routing
/// key clients pass in `InvokeRequest`), the owning extension's id,
/// the contribute id (the routing key the dispatcher uses), and the
/// resolved static description (loaded once at host startup, per R7).
#[derive(Debug, Clone)]
pub struct GrpcMethod {
    /// Owning extension's reverse-DNS id.
    pub extension: ExtensionId,
    /// `contributes.grpc[].id` — the dispatcher's routing key.
    pub contribute_id: String,
    /// gRPC service name from the manifest (e.g. `weather.v1.Weather`).
    pub service: String,
    /// gRPC method name from the manifest (e.g. `Current`).
    pub method: String,
    /// `true` for server-streaming methods (`InvokeStream` only).
    pub streaming: bool,
    /// Bundle-relative path of the `.proto` file. Surfaced so clients
    /// can locate the schema contract.
    pub proto_path: String,
    /// Static markdown description, read once at host startup. Bytes
    /// surfaced to clients via `ListMethods` are byte-identical to
    /// what the bundle ships (SCOPE R7 anti-prompt-injection
    /// guarantee).
    pub description: String,
}

/// Errors raised at adapter build time.
#[derive(Debug, thiserror::Error)]
pub enum BuildGrpcError {
    /// Two `contributes.grpc` entries declared the same
    /// `(service, method)` pair. Surfaces both registrants so the
    /// operator sees one diagnostic instead of an opaque "duplicate"
    /// failure at request time.
    #[error("(service, method) collision on `{service}/{method}` between {first:?} and {second:?}")]
    Collision {
        /// The gRPC service name involved in the collision.
        service: String,
        /// The gRPC method name involved in the collision.
        method: String,
        /// First registrant ("<extension>:<contribute_id>").
        first: String,
        /// Second registrant ("<extension>:<contribute_id>").
        second: String,
    },

    /// `description_file:` could not be read off disk.
    #[error("entry {entry:?}: reading description_file {path:?}: {source}")]
    DescriptionIo {
        /// "<extension>:<contribute_id>"
        entry: String,
        /// Manifest-relative description path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// `proto:` could not be located on disk. The adapter does not
    /// parse the proto in v0.1 (no runtime reflection), but it does
    /// verify the file exists so an extension that ships a manifest
    /// pointing at a missing `.proto` is caught at host startup
    /// rather than at the first client call.
    #[error("entry {entry:?}: proto file {path:?} not found: {source}")]
    ProtoMissing {
        /// "<extension>:<contribute_id>"
        entry: String,
        /// Manifest-relative proto path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Build one [`GrpcMethod`] per `contributes.grpc` entry.
///
/// `(service, method)` collisions across extensions are returned as
/// [`BuildGrpcError::Collision`] before any descriptor is produced —
/// the caller never sees a partial set.
pub fn build_grpc_methods(
    registry: &ExtensionRegistry,
) -> Result<Vec<GrpcMethod>, BuildGrpcError> {
    // First pass: collision detection on `(service, method)`. Done
    // before any I/O so a duplicate entry doesn't waste a `read` on
    // the second registrant's description file.
    let mut seen: HashMap<(String, String), String> = HashMap::new();
    let mut entries: Vec<(ExtensionId, PathBuf, ContributeGrpc)> = Vec::new();
    for record in registry.iter_validated() {
        if record.state != LifecycleState::Validated {
            continue;
        }
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        let Some(extension_id) = record.id.as_ref() else {
            continue;
        };
        for entry in &manifest.contributes.grpc {
            let key = (entry.service.clone(), entry.method.clone());
            let label = format!("{}:{}", extension_id.as_str(), entry.id);
            if let Some(prev) = seen.get(&key) {
                return Err(BuildGrpcError::Collision {
                    service: entry.service.clone(),
                    method: entry.method.clone(),
                    first: prev.clone(),
                    second: label,
                });
            }
            seen.insert(key, label);
            entries.push((extension_id.clone(), record.bundle_dir.clone(), entry.clone()));
        }
    }

    // Second pass: resolve descriptions + verify proto files exist.
    let mut out: Vec<GrpcMethod> = Vec::with_capacity(entries.len());
    for (ext_id, bundle, entry) in entries {
        let entry_label = format!("{}:{}", ext_id.as_str(), entry.id);

        let proto_path = bundle.join(&entry.proto);
        std::fs::metadata(&proto_path).map_err(|source| BuildGrpcError::ProtoMissing {
            entry: entry_label.clone(),
            path: entry.proto.clone(),
            source,
        })?;

        let desc_path = bundle.join(&entry.description_file);
        let description =
            std::fs::read_to_string(&desc_path).map_err(|source| BuildGrpcError::DescriptionIo {
                entry: entry_label.clone(),
                path: entry.description_file.clone(),
                source,
            })?;

        out.push(GrpcMethod {
            extension: ext_id,
            contribute_id: entry.id,
            service: entry.service,
            method: entry.method,
            streaming: entry.streaming,
            proto_path: entry.proto,
            description,
        });
    }
    Ok(out)
}
