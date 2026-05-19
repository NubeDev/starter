//! [`ExtensionGrpcService`] — concrete tonic implementation of the
//! `starter.ext.grpc.v1.ExtensionGrpc` backplane service.
//!
//! Routes inbound `Invoke` / `InvokeStream` calls by their
//! `(service, method)` pair to the matching extension's
//! `(extension_id, contribute_id)` and dispatches through
//! [`GrpcDispatcher`]. Cancellation: when the client closes the
//! server-streaming response (HTTP/2 RST_STREAM), the dispatcher's
//! [`crate::CancelHandle`] is dropped in the background task, which
//! fires the kernel's `stream.cancel`.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::stream::Stream;
use futures::StreamExt;
use tonic::{Request, Response, Status};

use crate::adapter::GrpcMethod;
use crate::dispatcher::{DispatchError, GrpcDispatcher};
use crate::proto::extension_grpc_server::{ExtensionGrpc, ExtensionGrpcServer};
use crate::proto::{
    InvokeRequest, InvokeResponse, InvokeStreamEvent, ListMethodsRequest, ListMethodsResponse,
    MethodDescriptor,
};

/// Type alias for the streaming response item the tonic codegen
/// expects.
type StreamItem = Result<InvokeStreamEvent, Status>;

/// Concrete tonic service: routes `(service, method)` → manifest
/// entry → dispatcher.
pub struct ExtensionGrpcService {
    /// Lookup table from `(service, method)` to the descriptor.
    by_pair: HashMap<(String, String), GrpcMethod>,
    /// All descriptors, kept for `ListMethods`.
    methods: Vec<GrpcMethod>,
    dispatcher: Arc<dyn GrpcDispatcher>,
    default_timeout: Duration,
}

impl ExtensionGrpcService {
    /// Build a service from the adapter-produced method descriptors,
    /// the consumer's dispatcher, and a default per-call timeout
    /// applied when the inbound request carries no `grpc-timeout`
    /// header.
    pub fn new(
        methods: Vec<GrpcMethod>,
        dispatcher: Arc<dyn GrpcDispatcher>,
        default_timeout: Duration,
    ) -> Self {
        let by_pair = methods
            .iter()
            .map(|m| ((m.service.clone(), m.method.clone()), m.clone()))
            .collect();
        Self {
            by_pair,
            methods,
            dispatcher,
            default_timeout,
        }
    }

    /// Wrap in the tonic-generated server ready for
    /// `Server::builder().add_service(…)`.
    pub fn into_server(self) -> ExtensionGrpcServer<Self> {
        ExtensionGrpcServer::new(self)
    }

    /// Resolve a `(service, method)` pair to the manifest entry or
    /// return a tonic `NOT_FOUND` status.
    fn resolve(&self, service: &str, method: &str) -> Result<&GrpcMethod, Status> {
        self.by_pair.get(&(service.to_owned(), method.to_owned())).ok_or_else(|| {
            Status::not_found(format!(
                "no extension method registered for ({service:?}, {method:?})"
            ))
        })
    }

    fn parse_input(args_proto_json: &str) -> Result<serde_json::Value, Status> {
        if args_proto_json.is_empty() {
            Ok(serde_json::Value::Null)
        } else {
            serde_json::from_str(args_proto_json).map_err(|e| {
                Status::invalid_argument(format!("args_proto_json is not valid JSON: {e}"))
            })
        }
    }
}

#[tonic::async_trait]
impl ExtensionGrpc for ExtensionGrpcService {
    async fn list_methods(
        &self,
        _req: Request<ListMethodsRequest>,
    ) -> Result<Response<ListMethodsResponse>, Status> {
        let methods = self
            .methods
            .iter()
            .map(|m| MethodDescriptor {
                extension_id: m.extension.as_str().to_owned(),
                contribute_id: m.contribute_id.clone(),
                service: m.service.clone(),
                method: m.method.clone(),
                streaming: m.streaming,
                description: m.description.clone(),
                proto_path: m.proto_path.clone(),
            })
            .collect();
        Ok(Response::new(ListMethodsResponse { methods }))
    }

    async fn invoke(
        &self,
        req: Request<InvokeRequest>,
    ) -> Result<Response<InvokeResponse>, Status> {
        let timeout = self.default_timeout;
        let InvokeRequest {
            service,
            method,
            args_proto_json,
        } = req.into_inner();
        let entry = self.resolve(&service, &method)?.clone();
        if entry.streaming {
            return Err(Status::failed_precondition(format!(
                "({service:?}, {method:?}) is server-streaming; use InvokeStream"
            )));
        }
        let input = Self::parse_input(&args_proto_json)?;
        let result = self
            .dispatcher
            .dispatch(&entry.extension, &entry.contribute_id, input, timeout)
            .await
            .map_err(dispatch_to_status)?;
        Ok(Response::new(InvokeResponse {
            result_proto_json: serde_json::to_string(&result)
                .map_err(|e| Status::internal(format!("encoding response: {e}")))?,
        }))
    }

    type InvokeStreamStream = Pin<Box<dyn Stream<Item = StreamItem> + Send + 'static>>;

    async fn invoke_stream(
        &self,
        req: Request<InvokeRequest>,
    ) -> Result<Response<Self::InvokeStreamStream>, Status> {
        let timeout = self.default_timeout;
        let InvokeRequest {
            service,
            method,
            args_proto_json,
        } = req.into_inner();
        let entry = self.resolve(&service, &method)?.clone();
        if !entry.streaming {
            return Err(Status::failed_precondition(format!(
                "({service:?}, {method:?}) is unary; use Invoke"
            )));
        }
        let input = Self::parse_input(&args_proto_json)?;

        let response = self
            .dispatcher
            .dispatch_stream(&entry.extension, &entry.contribute_id, input, timeout)
            .await
            .map_err(dispatch_to_status)?;

        // Map the kernel event stream into tonic frames. The cancel
        // handle rides along inside the stream's state — dropping
        // the response (which tonic does when the client disconnects)
        // drops the handle, which fires `stream.cancel`. This is the
        // same shape the CLI adapter uses on SIGINT.
        let crate::dispatcher::StreamResponse {
            events, cancel, ..
        } = response;
        let cancel = Arc::new(cancel);
        let mapped = events.map(move |item| {
            // Keep `cancel` alive for the lifetime of the stream by
            // capturing it; explicit `let _ =` prevents the closure
            // from logically dropping it on the first frame.
            let _keep_alive = &cancel;
            match item {
                Ok(ev) => {
                    let payload_proto_json = serde_json::to_string(&ev)
                        .unwrap_or_else(|e| format!(r#"{{"_encode_error":"{e}"}}"#));
                    Ok(InvokeStreamEvent { payload_proto_json })
                }
                Err(e) => Err(Status::internal(format!("kernel stream error: {e}"))),
            }
        });

        Ok(Response::new(Box::pin(mapped) as Self::InvokeStreamStream))
    }
}

fn dispatch_to_status(err: DispatchError) -> Status {
    Status::new(err.tonic_code(), err.to_string())
}

/// One-line convenience for the common path: build the service +
/// wrap it in the tonic-generated server.
pub fn extension_grpc_server(
    methods: Vec<GrpcMethod>,
    dispatcher: Arc<dyn GrpcDispatcher>,
    default_timeout: Duration,
) -> ExtensionGrpcServer<ExtensionGrpcService> {
    ExtensionGrpcService::new(methods, dispatcher, default_timeout).into_server()
}
